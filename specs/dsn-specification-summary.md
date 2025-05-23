# DSN Specification Summary

## Executive Summary

The Subspace Distributed Storage Network (DSN) is a two-layer decentralized storage system designed to permanently store and efficiently retrieve blockchain history. This specification documents the current implementation in the Subspace codebase.

## Architecture Summary

### Two-Layer Design

1. **L1 - Archival Storage Layer**
   - **Purpose**: Permanent storage of encoded pieces
   - **Location**: Farmer plots on disk
   - **Latency**: ~1 second (due to decoding)
   - **Reliability**: High (permanent storage)

2. **L2 - Cache Layer**
   - **Purpose**: Fast retrieval of frequently accessed pieces
   - **Location**: Farmer memory/SSD cache
   - **Latency**: 10-100ms
   - **Capacity**: Small percentage of farmer storage

### Key Properties

- **Permissionless**: Anyone can join as a farmer
- **Verifiable**: Cryptographic commitments ensure data integrity
- **Durable**: Redundant storage across many farmers
- **Uniform**: Proximity-based distribution ensures even spread
- **Retrievable**: Multi-tier retrieval with fallback mechanisms

## Core Components

### 1. Data Structures
- **Piece**: 1 MiB (1,048,576 bytes) unit of storage
- **Segment**: Collection of pieces with metadata
- **PieceIndex**: Unique identifier for each piece
- **GlobalObject**: Maps objects to piece locations

### 2. Networking Protocols
- **PieceByIndexRequest/Response**: L1 retrieval
- **CachedPieceByIndexRequest/Response**: L2 retrieval
- **SegmentHeaderRequest/Response**: Metadata retrieval

### 3. Key Algorithms
- **Proximity Calculation**: XOR distance between peer ID and piece
- **Cache Selection**: Closest pieces by proximity
- **Random Walk**: Discovery mechanism when direct retrieval fails
- **LRU Eviction**: Cache management with proximity weighting

### 4. Retrieval Flow
```
1. Request piece by index
2. Try L2 cache (fast path)
   - Success: Return immediately
   - Fail: Continue to L1
3. Try L1 connected peers
   - Success: Decode and return
   - Fail: Random walk
4. Random walk discovery
   - Query random peers
   - Find piece holders
5. Validate and return
```

## Implementation Status

### ✅ Fully Implemented
- L1 archival storage in farmer plots
- L2 caching with proximity-based selection
- Multi-tier retrieval with fallbacks
- Piece validation via KZG commitments
- Prometheus metrics for monitoring

### 🚧 Partially Implemented
- Object storage (blockchain data only)
- Advanced caching strategies
- Performance optimizations

## Key Metrics

### Performance
- **L1 Retrieval**: ~1 second average
- **L2 Cache Hit**: 10-100ms
- **L2 Cache Miss to L1**: 1-2 seconds total

### Configuration
- **Piece Size**: 1 MiB
- **Cache Batch Size**: 256 pieces
- **Concurrent Downloads**: 10 (default)
- **Random Walk Rounds**: 3 (default)

## Security Model

### Protections
- **Data Integrity**: KZG commitments and Blake3 hashes
- **Sybil Resistance**: Proof-of-Space farming
- **Byzantine Tolerance**: Redundant storage across farmers
- **Censorship Resistance**: Decentralized architecture

### Trust Assumptions
- Honest majority of farmers
- Sufficient network connectivity
- Correct implementation of cryptographic primitives

## For Implementers

### Quick Start
1. Study the [Implementation Guide](./dsn-implementation-guide.md)
2. Review [Data Structures](./dsn-data-structures.md)
3. Understand [Algorithms](./dsn-algorithms.md)
4. Follow [Protocols](./dsn-protocols-specification.md)

### Key Interfaces
```rust
// Piece retrieval
trait PieceGetter {
    async fn get_piece(&self, index: PieceIndex) -> Option<Piece>;
}

// Cache management
trait PieceCache {
    async fn write_piece(&self, offset: PieceCacheOffset, index: PieceIndex, piece: &Piece) -> Result<()>;
    async fn read_piece(&self, offset: PieceCacheOffset) -> Result<Option<Piece>>;
}

// Object fetching
trait ObjectFetcher {
    async fn fetch_object(&self, mapping: GlobalObject) -> Result<Vec<u8>>;
}
```

## Future Considerations

While not part of the current implementation, potential enhancements include:
- Predictive caching based on access patterns
- Compression for network efficiency
- Advanced peer selection strategies
- Extended object support (>248MB)

## Conclusion

The Subspace DSN provides a robust, decentralized storage layer for blockchain data with strong guarantees for permanence and retrievability. The two-layer architecture balances performance with reliability, while the proximity-based distribution ensures scalability as the network grows. 