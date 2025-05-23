# DSN Usage Patterns in Subspace

## Overview

The Distributed Storage Network (DSN) is a critical component of the Subspace blockchain that provides decentralized storage and retrieval of blockchain history. This document details how various services and components in the Subspace ecosystem utilize DSN.

## Core DSN Architecture

DSN operates as a two-layer storage system:

1. **L1 - Archival Storage Layer**: Permanent storage in farmer plots with ~1 second retrieval latency
2. **L2 - Cache Layer**: Fast in-memory/SSD cache with 10-100ms retrieval latency

## DSN Usage by Service

### 1. Node Synchronization

The node uses DSN for blockchain synchronization in several scenarios:

#### Sync from DSN (`subspace-service/src/sync_from_dsn/`)

**Purpose**: Synchronize blockchain state when traditional P2P sync is unavailable or inefficient

**Key Components**:
- `DsnPieceGetter`: Wrapper around `PieceProvider` implementing the `PieceGetter` trait
- `import_blocks_from_dsn()`: Reconstructs and imports blocks from archived segments
- `SegmentHeaderDownloader`: Downloads segment headers from the network

**Flow**:
1. Node detects it's offline or behind (>10 minutes without new blocks)
2. Downloads segment headers from DSN
3. Retrieves pieces for segments containing blocks to sync
4. Reconstructs blocks from pieces
5. Imports blocks through the standard import queue

```rust
// From sync_from_dsn.rs
pub struct DsnPieceGetter<PV: PieceValidator>(PieceProvider<PV>);

#[async_trait]
impl<PV> PieceGetter for DsnPieceGetter<PV> {
    async fn get_piece(&self, piece_index: PieceIndex) -> anyhow::Result<Option<Piece>> {
        Ok(self.0.get_piece_from_cache(piece_index).await)
    }
}
```

#### Snap Sync (`subspace-service/src/sync_from_dsn/snap_sync.rs`)

**Purpose**: Fast initial sync from genesis or specific block

**Process**:
1. Downloads segment headers to determine target segment
2. Retrieves all pieces for the target segment
3. Reconstructs blocks from segment data
4. Imports state and blocks

**Usage Conditions**:
- Only works from genesis state (current limitation)
- Requires node to pause regular sync
- Used for quick bootstrapping of new nodes

### 2. Farmer Operations

Farmers are the primary participants in DSN, both storing and retrieving pieces.

#### Piece Retrieval Hierarchy (`subspace-farmer/src/farmer_piece_getter.rs`)

Farmers use a multi-tier approach for piece retrieval:

1. **Local Farmer Cache** (fastest)
2. **DSN L2 Cache** (fast, network-based)
3. **Node RPC** (before going to L1)
4. **Local Plot Storage** (L1, requires decoding)
5. **DSN L1 Archival Storage** (slowest, network + decoding)

```rust
// Retrieval flow from farmer_piece_getter.rs
async fn get_piece_fast_internal(&self, piece_index: PieceIndex) -> Option<Piece> {
    // 1. Try farmer cache
    if let Some(piece) = self.farmer_caches.get_piece(piece_index.to_multihash()).await {
        return Some(piece);
    }
    
    // 2. Try DSN L2 cache
    if let Some(piece) = self.piece_provider.get_piece_from_cache(piece_index).await {
        self.farmer_caches.maybe_store_additional_piece(piece_index, &piece).await;
        return Some(piece);
    }
    
    // 3. Try node RPC
    if let Ok(Some(piece)) = self.node_client.piece(piece_index).await {
        self.farmer_caches.maybe_store_additional_piece(piece_index, &piece).await;
        return Some(piece);
    }
    
    None
}
```

#### Cache Management (`subspace-farmer/src/farmer_cache.rs`)

**Purpose**: Manage L2 cache distribution based on proximity

**Key Features**:
- Proximity-based piece selection using XOR distance
- Automatic synchronization with new segments
- LRU eviction with proximity weighting
- Support for both dedicated cache and plot cache

**Cache Types**:
1. **Piece Cache**: Dedicated cache storage
2. **Plot Cache**: Utilizes unused plot space for additional caching

### 3. Plotting Process

Farmers use DSN to retrieve pieces needed for plotting sectors.

#### Piece Fetching for Plotting

**Configuration**:
- Uses retry policy with exponential backoff
- Multiple retrieval attempts to handle network issues
- Concurrent piece downloads for efficiency

```rust
// From bin/subspace-farmer/commands/farm.rs
let piece_getter = FarmerPieceGetter::new(
    piece_provider,
    farmer_caches,
    node_client.clone(),
    Arc::clone(&plotted_pieces),
    DsnCacheRetryPolicy {
        max_retries: PIECE_GETTER_MAX_RETRIES,
        backoff: ExponentialBackoff {
            initial_interval: GET_PIECE_INITIAL_INTERVAL,
            max_interval: GET_PIECE_MAX_INTERVAL,
            max_elapsed_time: None,
            multiplier: 1.75,
            ..ExponentialBackoff::default()
        },
    },
);
```

### 4. Gateway Service

The Gateway provides HTTP/RPC access to data stored in DSN.

#### Object Retrieval (`subspace-gateway/`)

**Purpose**: Fetch and reconstruct objects (e.g., transactions, blocks) from DSN

**Components**:
- `DsnPieceGetter`: Gateway-specific piece getter
- `ObjectFetcher`: Reconstructs objects from pieces
- `SegmentCommitmentPieceValidator`: Validates retrieved pieces

**Flow**:
1. Receive object request with mapping information
2. Determine required pieces from mapping
3. Fetch pieces from DSN (L2 cache first, then L1)
4. Reconstruct and validate object
5. Return object data

```rust
// Gateway piece retrieval strategy
const MAX_RANDOM_WALK_ROUNDS: usize = 15;

impl<PV> PieceGetter for DsnPieceGetter<PV> {
    async fn get_piece(&self, piece_index: PieceIndex) -> anyhow::Result<Option<Piece>> {
        // Try L2 cache first
        if let Some(piece) = self.get_from_cache([piece_index]).await {
            return Ok(Some(piece));
        }
        
        // Fall back to L1 archival storage
        Ok(self.get_piece_from_archival_storage(piece_index, MAX_RANDOM_WALK_ROUNDS).await)
    }
}
```

### 5. RPC Services

Node RPC endpoints expose DSN functionality to external clients.

#### Subspace RPC (`sc-consensus-subspace-rpc/`)

**Endpoints**:
- `subspace_piece`: Retrieve individual pieces
- `subspace_segmentHeaders`: Get segment metadata
- `subspace_lastSegmentHeaders`: Get recent segment headers
- Object mapping subscriptions for real-time updates

**Special Handling**:
- Genesis segment recreation on-demand
- In-memory caching of recent archived segments
- Piece validation before returning to clients

### 6. Archiver

The archiver creates the data that gets stored in DSN.

#### Archival Process (`sc-consensus-subspace/src/archiver.rs`)

**Purpose**: Convert blockchain blocks into archived segments

**Integration with DSN**:
1. Archives blocks at confirmation depth
2. Creates segment headers for DSN storage
3. Notifies DSN participants of new segments
4. Maintains `SegmentHeadersStore` for validation

## DSN Protocol Usage

### Request/Response Protocols

Different services use different DSN protocols based on their needs:

1. **PieceByIndexRequest/Response**: Used for L1 retrieval by all services
2. **CachedPieceByIndexRequest/Response**: Used for L2 cache queries
3. **SegmentHeaderRequest/Response**: Used for metadata synchronization

### Piece Validation

All services implement piece validation to ensure data integrity:

1. **Farmers**: Use `SegmentCommitmentPieceValidator` with local segment headers
2. **Gateway**: Validates against segment headers from node RPC
3. **Node**: Validates during sync using KZG commitments

## Performance Considerations

### Connection Management

Services configure connection limits based on their role:

```rust
// Typical configuration
const PIECE_PROVIDER_MULTIPLIER: usize = 10;
let piece_provider = PieceProvider::new(
    node.clone(),
    validator,
    Arc::new(Semaphore::new(out_connections * PIECE_PROVIDER_MULTIPLIER)),
);
```

### Retry Strategies

Different services use different retry policies:

- **Farmers**: Aggressive retry with exponential backoff for plotting
- **Node Sync**: Limited retries to avoid blocking consensus
- **Gateway**: Moderate retry for user-facing requests

### Caching Strategies

1. **Write-through**: Pieces fetched from network are cached locally
2. **Proximity-based**: Cache pieces close to peer ID in XOR space
3. **LRU with weights**: Evict least recently used, considering proximity

## Error Handling

Common error scenarios and handling:

1. **Piece Not Found**: Try alternative sources (L2 → L1 → random walk)
2. **Validation Failure**: Ban peer and retry with different source
3. **Timeout**: Use exponential backoff and retry
4. **Network Issues**: Fall back to alternative retrieval methods

## Monitoring and Metrics

Services expose metrics for DSN operations:

- Request counts by type and result
- Retrieval latencies (p50, p95, p99)
- Cache hit/miss rates
- Bandwidth usage
- Validation failures
