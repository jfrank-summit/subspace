# DSN Implementation - Key Findings

## Overview

The Distributed Storage Network (DSN) in Subspace is a two-layered storage system designed to provide permanent, verifiable, and efficiently retrievable storage for blockchain data. The implementation focuses on storing and serving pieces of the blockchain history.

## Current Implementation Status

### ✅ Implemented

1. **L1 - Archival Storage Layer**
   - Fully implemented in farmer plots
   - Pieces are erasure-coded and masked
   - Random walk algorithm for piece discovery
   - Gradual plot expiration for uniform distribution
   - ~1 second retrieval latency due to decoding

2. **L2 - Pieces Cache Layer**
   - Implemented as `FarmerCache` in `subspace-farmer`
   - DHT-based proximity storage
   - Near-instantaneous retrieval
   - Automatic synchronization with new segments
   - Smart piece selection based on peer proximity

3. **Core Networking Infrastructure**
   - Request/response protocols for piece retrieval
   - Kademlia DHT for peer discovery
   - Connection management and limits
   - Piece validation framework

4. **Data Retrieval APIs**
   - `ObjectFetcher` for reconstructing objects from pieces
   - `PieceProvider` for various retrieval strategies
   - Segment downloading capabilities
   - Concurrent piece fetching with cancellation

5. **Metrics and Monitoring**
   - Prometheus-compatible metrics
   - Cache hit/miss/error rates
   - Capacity utilization tracking
   - Per-operation counters

### 🚧 Partially Implemented

1. **Object Storage for Blockchain Data**
   - Object mapping infrastructure exists
   - Framework for `GlobalObjectMapping` in place
   - Used for blockchain data archival

2. **Performance Optimizations**
   - Basic retry logic and concurrent downloads
   - Connection reuse partially implemented
   - Advanced caching strategies in development

## Key Design Patterns

### 1. Proximity-Based Storage
- Uses `KeyWithDistance` to determine which farmers store which pieces
- Ensures uniform distribution across the network
- Enables efficient DHT-based discovery

### 2. Layered Retrieval Strategy
```
L2 Cache Hit → Return immediately
    ↓ (miss)
L1 Connected Peers → Try direct retrieval
    ↓ (fail)
L1 Random Walk → Discover via DHT
```

### 3. Piece Validation Architecture
- `PieceValidator` trait for extensible validation
- Currently uses commitment verification
- Prevents accepting corrupted/malicious data

### 4. Worker Pattern
- `FarmerCacheWorker` handles background synchronization
- Subscribes to new segment notifications
- Maintains cache consistency automatically

## Protocol Details

### Request Types

1. **CachedPieceByIndexRequest**
   - Used for L2 cache queries
   - Includes "interested pieces" for efficiency
   - Returns multiple pieces if available

2. **PieceByIndexRequest**
   - Used for L1 archival retrieval
   - Single piece requests
   - Triggers plot decoding

3. **SegmentHeaderRequest**
   - Retrieves segment metadata
   - Used for synchronization

## Configuration Parameters

### Key Constants
- `SYNC_BATCH_SIZE`: 256 pieces
- `SYNC_CONCURRENT_BATCHES`: 4
- `PIECE_PROVIDER_MULTIPLIER`: 10
- `INTERMEDIATE_CACHE_UPDATE_INTERVAL`: 100 pieces
- `CachedPieceByIndexRequest::RECOMMENDED_LIMIT`: Protocol-specific limit

### Dynamic Configuration
- Cache sizes based on farmer capacity
- Connection limits (in/out)
- Retry policies
- Timeouts

## Metrics and Monitoring

### Available Metrics

#### Farmer Cache Metrics (`farmer_cache` prefix)
- `cache_get_hit`: Successful cache retrievals
- `cache_get_miss`: Cache misses requiring fallback
- `cache_get_error`: Failed cache operations
- `cache_find_hit`: Successful piece location queries
- `cache_find_miss`: Failed piece location queries
- `piece_cache_capacity_total`: Total cache capacity in pieces
- `piece_cache_capacity_used`: Currently used cache capacity

#### Disk Piece Cache Metrics
- `write_piece`: Piece write operations
- `read_piece`: Piece read operations
- `read_piece_index`: Piece index read operations

### Monitoring Insights
- Cache hit rate: `cache_get_hit / (cache_get_hit + cache_get_miss)`
- Cache utilization: `piece_cache_capacity_used / piece_cache_capacity_total`
- Error rate: `cache_get_error / total_requests`

## Integration Points

### For Applications
- `ObjectFetcher` - High-level API for data retrieval
- `PieceGetter` trait - Abstract interface for piece retrieval
- RPC endpoints for object mappings

### For Farmers
- `FarmerCache` - L2 cache management
- Plot storage - L1 archival layer
- Request handlers for serving pieces

### For Nodes
- Archiver integration for creating segments
- Object mapping notifications
- Segment header distribution

## Performance Characteristics

### L1 (Archival Storage)
- **Latency**: ~1 second due to plot decoding
- **Reliability**: High (permanent storage)
- **Capacity**: Full blockchain history distributed across farmers

### L2 (Cache Layer)
- **Latency**: 10-100ms (memory/SSD access)
- **Hit Rate**: Varies based on proximity and popularity
- **Capacity**: Small percentage of farmer storage

## Security Considerations

### Current Protections
- Piece validation via KZG commitments
- Blake3 hash verification for objects
- Byzantine fault tolerance through redundancy
- Sybil resistance through Proof-of-Space

### Security Properties
- Data integrity guaranteed by cryptographic commitments
- Availability ensured through redundant storage
- Censorship resistance via decentralized architecture

## Recommendations for Specification

1. **Formalize Protocol Messages**
   - Define exact wire formats
   - Specify version negotiation
   - Document error codes

2. **Define Performance Targets**
   - L1 retrieval SLAs
   - L2 cache hit ratios
   - Network bandwidth limits

3. **Standardize Metrics**
   - Define required metrics
   - Specify aggregation rules
   - Create alerting guidelines

4. **Document Algorithms**
   - Piece distribution algorithm details
   - Cache eviction policies
   - Proximity calculations 