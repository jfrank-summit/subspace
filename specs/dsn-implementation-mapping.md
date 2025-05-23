# DSN Implementation Mapping

This document maps the conceptual DSN architecture to the actual implementation in the codebase.

## Layer Mapping

### L1 - Archival Storage Layer
**Implementation Location**: Farmer plots (permanent storage)

**Key Components**:
- Plot storage in farmers (actual data storage)
- `PieceByIndexRequest/Response` protocol for retrieval
- Random walk algorithm for piece discovery
- Implementation in `subspace-networking/src/utils/piece_provider.rs`:
  - `get_piece_from_archival_storage()` - L1 retrieval logic
  - `get_piece_by_random_walking()` - Discovery via random walk

**Characteristics**:
- Pieces are erasure-coded and masked
- ~1 second retrieval time due to decoding
- Uses Kademlia DHT for peer discovery
- Gradual plot expiration for uniform distribution

### L2 - Pieces Cache Layer  
**Implementation Location**: `subspace-farmer/src/farmer_cache.rs`

**Key Components**:
- `FarmerCache` - Main L2 cache implementation
- `PieceCachesState` - Cache state management
- `CachedPieceByIndexRequest/Response` protocol
- DHT-based piece distribution using peer proximity

**Characteristics**:
- Stores unencoded pieces for fast retrieval
- Near-instantaneous retrieval
- Small percentage of farmer's storage
- Proximity-based storage using `KeyWithDistance`

## Core Components

### 1. Networking Layer (`subspace-networking`)

**Purpose**: Provides DSN networking infrastructure

**Key Files**:
- `src/lib.rs` - Public API
- `src/node.rs` - Node implementation
- `src/protocols/request_response/` - Request handlers
- `src/utils/piece_provider.rs` - Piece retrieval logic

**Protocols**:
- `PieceByIndexRequest` - L1 retrieval
- `CachedPieceByIndexRequest` - L2 retrieval  
- `SegmentHeaderRequest` - Segment metadata

### 2. Farmer Cache (`subspace-farmer`)

**Purpose**: Implements L2 caching layer

**Key Components**:
- `FarmerCache` - Main cache structure
- `FarmerCacheWorker` - Background worker for cache maintenance
- `PlotCaches` - Additional caching for farming operations

**Cache Types**:
- Node cache: Recent archived segments
- Farmer cache: L2 proximity-based cache
- Plot cache: Farming-specific cache

### 3. Data Retrieval (`subspace-data-retrieval`)

**Purpose**: High-level data fetching APIs

**Key Components**:
- `ObjectFetcher` - Reconstructs objects from pieces
- `PieceGetter` trait - Abstract piece retrieval
- `SegmentDownloading` - Full segment retrieval

## Request Flow

### L2 Cache Hit Flow:
1. Client requests piece via `CachedPieceByIndexRequest`
2. Request routed to farmers with piece in L2 cache
3. Farmer returns unencoded piece immediately
4. ~10-100ms typical latency

### L1 Retrieval Flow:
1. L2 cache miss triggers L1 retrieval
2. `get_piece_from_archival_storage()` called
3. First tries connected peers
4. Falls back to random walk if needed
5. Farmer decodes piece from plot
6. ~1 second typical latency

### Object Retrieval Flow:
1. Application requests object by hash
2. `ObjectFetcher` determines required pieces
3. Pieces retrieved via L2/L1 as needed
4. Object reconstructed and validated
5. Hash verification ensures integrity

## Configuration Parameters

### Network Level:
- Connection limits (in/out connections)
- Request timeout values
- Retry policies
- Piece provider concurrency (`PIECE_PROVIDER_MULTIPLIER`)

### Cache Level:
- Cache capacity per farmer
- Sync batch size (`SYNC_BATCH_SIZE`)
- Update intervals (`INTERMEDIATE_CACHE_UPDATE_INTERVAL`)
- Proximity threshold for piece storage

### Protocol Level:
- Maximum pieces per request
- Request/response size limits
- Validation requirements 