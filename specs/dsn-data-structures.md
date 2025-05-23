# DSN Data Structures Specification

## 1. Overview

This document specifies the core data structures used in the Subspace Distributed Storage Network. All structures use SCALE codec for serialization unless otherwise specified.

## 2. Basic Types

### 2.1 PieceIndex

A unique identifier for each piece in the network.

```rust
struct PieceIndex(u64);
```

📍 **Source**: [`crates/subspace-core-primitives/src/pieces.rs:61`](https://github.com/autonomys/subspace/blob/main/crates/subspace-core-primitives/src/pieces.rs#L61)

**Properties:**
- Monotonically increasing
- Source pieces have specific positions
- Parity pieces are derived from source pieces

**Methods:**
```rust
impl PieceIndex {
    /// Check if this is a source piece (not parity)
    pub fn is_source(&self) -> bool;
    
    /// Get position within segment (for source pieces)
    pub fn source_position(&self) -> u32;
    
    /// Get next source piece index
    pub fn next_source_index(&self) -> PieceIndex;
    
    /// Convert to multihash for DHT operations
    pub fn to_multihash(&self) -> Multihash;
}
```

### 2.2 SegmentIndex

Identifier for archived segments.

```rust
struct SegmentIndex(u64);
```

📍 **Source**: [`crates/subspace-core-primitives/src/segments.rs:56`](https://github.com/autonomys/subspace/blob/main/crates/subspace-core-primitives/src/segments.rs#L56)

**Constants:**
```rust
impl SegmentIndex {
    pub const ZERO: Self = Self(0);
}
```

### 2.3 Blake3Hash

256-bit Blake3 hash used for content addressing.

```rust
struct Blake3Hash([u8; 32]);
```

📍 **Source**: [`crates/subspace-core-primitives/src/hashes.rs`](https://github.com/autonomys/subspace/blob/main/crates/subspace-core-primitives/src/hashes.rs)

## 3. Piece Structure

### 3.1 Piece

The fundamental unit of data storage in DSN.

```rust
struct Piece([u8; Piece::SIZE]);

impl Piece {
    /// Exactly 1 MiB
    pub const SIZE: usize = 1_048_576;
}
```

📍 **Source**: [`crates/subspace-core-primitives/src/pieces.rs:1004`](https://github.com/autonomys/subspace/blob/main/crates/subspace-core-primitives/src/pieces.rs#L1004)

**Properties:**
- Fixed size: 1,048,576 bytes (1 MiB)
- Contains either raw data (L2) or encoded data (L1)
- Validated using KZG commitment

### 3.2 RawRecord

The underlying data structure before encoding.

```rust
struct RawRecord([u8; RawRecord::SIZE]);

impl RawRecord {
    pub const SIZE: usize = 1_048_576;
    
    /// Convert to raw record chunks for processing
    pub fn to_raw_record_chunks(&self) -> impl Iterator<Item = &[u8]>;
}
```

📍 **Source**: [`crates/subspace-core-primitives/src/pieces.rs`](https://github.com/autonomys/subspace/blob/main/crates/subspace-core-primitives/src/pieces.rs)

### 3.3 Record

Encoded piece with commitment.

```rust
struct Record {
    data: Vec<u8>,
    commitment: RecordCommitment,
}
```

📍 **Source**: [`crates/subspace-core-primitives/src/pieces.rs`](https://github.com/autonomys/subspace/blob/main/crates/subspace-core-primitives/src/pieces.rs)

## 4. Segment Structure

### 4.1 RecordedHistorySegment

A segment containing multiple pieces of archived history.

```rust
struct RecordedHistorySegment {
    pieces: Vec<Piece>,
}

impl RecordedHistorySegment {
    /// Total segment size in bytes
    pub const SIZE: usize = /* implementation defined */;
    
    /// Number of pieces per segment
    pub const NUM_PIECES: usize = /* implementation defined */;
}
```

📍 **Source**: [`crates/subspace-core-primitives/src/segments.rs`](https://github.com/autonomys/subspace/blob/main/crates/subspace-core-primitives/src/segments.rs)

### 4.2 SegmentHeader

Metadata about an archived segment.

```rust
struct SegmentHeader {
    /// Version of the segment format
    version: SegmentVersion,
    
    /// Index of this segment
    segment_index: SegmentIndex,
    
    /// Merkle root of segment pieces
    segment_commitment: SegmentCommitment,
    
    /// Hash of previous segment header
    prev_segment_header_hash: Blake3Hash,
    
    /// Last archived block
    last_archived_block: LastArchivedBlock,
}

struct LastArchivedBlock {
    /// Block number
    number: u32,
    
    /// Block hash
    hash: Blake3Hash,
}
```

📍 **Source**: [`crates/subspace-core-primitives/src/segments.rs`](https://github.com/autonomys/subspace/blob/main/crates/subspace-core-primitives/src/segments.rs)

### 4.3 SegmentItem

Items within a segment (used during archiving).

```rust
enum SegmentItem {
    /// Segment header continuation
    ParentSegmentHeader(SegmentHeader),
    
    /// Block data
    Block(BlockData),
    
    /// Transaction data
    Transaction(TransactionData),
    
    /// Other archivable items
    Other(Vec<u8>),
}
```

📍 **Source**: [`crates/subspace-archiving/src/archiver.rs`](https://github.com/autonomys/subspace/blob/main/crates/subspace-archiving/src/archiver.rs)

## 5. Object Mapping

### 5.1 GlobalObject

Mapping of an object to its location in pieces.

```rust
struct GlobalObject {
    /// Source piece index containing the object start
    pub piece_index: PieceIndex,
    
    /// Offset within the piece where object starts
    pub offset: u32,
}
```

📍 **Source**: [`crates/subspace-core-primitives/src/objects.rs`](https://github.com/autonomys/subspace/blob/main/crates/subspace-core-primitives/src/objects.rs)

**Constraints:**
- `piece_index` must be a source piece
- `offset` must be less than `RawRecord::SIZE`

### 5.2 GlobalObjectMapping

Collection of objects and their mappings.

```rust
enum GlobalObjectMapping {
    /// Single object mapping
    Object(GlobalObject),
    
    /// Multiple object mappings
    Objects(Vec<GlobalObject>),
}

impl GlobalObjectMapping {
    /// Get all contained objects
    pub fn objects(&self) -> &[GlobalObject];
}
```

📍 **Source**: [`crates/subspace-core-primitives/src/objects.rs:116`](https://github.com/autonomys/subspace/blob/main/crates/subspace-core-primitives/src/objects.rs#L116)

### 5.3 BlockObject

Object representing archived block data.

```rust
struct BlockObject {
    /// Block hash
    pub hash: Blake3Hash,
    
    /// Mapping to piece location
    pub mapping: GlobalObject,
}
```

📍 **Source**: [`crates/subspace-core-primitives/src/objects.rs`](https://github.com/autonomys/subspace/blob/main/crates/subspace-core-primitives/src/objects.rs)

### 5.4 BlockObjectMapping

Mapping of blocks to their piece locations.

```rust
enum BlockObjectMapping {
    /// Version 0 mapping format
    V0(Vec<BlockObject>),
}
```

📍 **Source**: [`crates/subspace-core-primitives/src/objects.rs:32`](https://github.com/autonomys/subspace/blob/main/crates/subspace-core-primitives/src/objects.rs#L32)

## 6. Distance and Proximity

### 6.1 KeyWithDistance

Key with XOR distance calculation for DHT operations.

```rust
struct KeyWithDistance {
    key: Key,
    distance: Distance,
}

impl KeyWithDistance {
    /// Create new key with distance from peer ID
    pub fn new(peer_id: PeerId, target: Multihash) -> Self;
    
    /// Create from record key
    pub fn new_with_record_key(peer_id: PeerId, key: RecordKey) -> Self;
}
```

📍 **Source**: [`crates/subspace-networking/src/utils/key_with_distance.rs:10`](https://github.com/autonomys/subspace/blob/main/crates/subspace-networking/src/utils/key_with_distance.rs#L10)

**Distance Calculation:**
- XOR distance between peer ID and target
- Used for determining which farmers store which pieces
- Ensures uniform distribution

## 7. Cache-Specific Structures

### 7.1 PieceCacheOffset

Offset within a piece cache.

```rust
struct PieceCacheOffset(u32);
```

📍 **Source**: [`crates/subspace-farmer/src/farm.rs`](https://github.com/autonomys/subspace/blob/main/crates/subspace-farmer/src/farm.rs)

### 7.2 PieceCacheId

Unique identifier for a piece cache instance.

```rust
struct PieceCacheId(Uuid);
```

📍 **Source**: [`crates/subspace-farmer/src/farm.rs`](https://github.com/autonomys/subspace/blob/main/crates/subspace-farmer/src/farm.rs)

### 7.3 FarmerCacheOffset

Location within farmer's cache system.

```rust
struct FarmerCacheOffset {
    /// Which cache backend
    cache_index: u8,
    
    /// Offset within that cache
    piece_offset: PieceCacheOffset,
}
```

📍 **Source**: [`crates/subspace-farmer/src/farmer_cache.rs:62`](https://github.com/autonomys/subspace/blob/main/crates/subspace-farmer/src/farmer_cache.rs#L62)

## 8. Networking Structures

### 8.1 RecordKey

Key used in Kademlia DHT for piece lookups.

```rust
struct RecordKey(Vec<u8>);

impl From<PieceIndex> for RecordKey {
    fn from(piece_index: PieceIndex) -> Self {
        RecordKey(piece_index.to_multihash().to_bytes())
    }
}
```

📍 **Source**: libp2p's kad module

### 8.2 Multihash

Multi-format hash used in libp2p.

```rust
struct Multihash {
    code: u64,
    size: u8,
    digest: Vec<u8>,
}
```

📍 **Source**: [`crates/subspace-networking/src/utils/multihash.rs`](https://github.com/autonomys/subspace/blob/main/crates/subspace-networking/src/utils/multihash.rs)

## 9. Encoding and Serialization

### 9.1 SCALE Codec

All structures use SCALE (Simple Concatenated Aggregate Little-Endian) codec:

- Fixed-size arrays: Encoded as-is
- Vectors: Compact length prefix + elements
- Enums: Variant index (u8) + variant data
- Options: 0x00 for None, 0x01 + data for Some

### 9.2 Compact Encoding

Used for length prefixes and indices:

```
single-byte mode: 0b00 + 6-bit value (0-63)
two-byte mode: 0b01 + 14-bit value (64-16383)
four-byte mode: 0b10 + 30-bit value (16384-1073741823)
big-integer mode: 0b11 + bytes length + bytes
```

## 10. Constants

### 10.1 Size Constants

```rust
/// Size of a piece/record in bytes
const PIECE_SIZE: usize = 1_048_576; // 1 MiB

/// Size of Blake3 hash
const BLAKE3_HASH_SIZE: usize = 32;

/// Maximum segment padding
const MAX_SEGMENT_PADDING: usize = /* implementation defined */;
```

📍 **Source**: [`crates/subspace-core-primitives/src/pieces.rs`](https://github.com/autonomys/subspace/blob/main/crates/subspace-core-primitives/src/pieces.rs)

### 10.2 Limits

```rust
/// Maximum object size that can be reliably retrieved
const MAX_SUPPORTED_OBJECT_LENGTH: usize = /* see object_fetcher */;

/// Recommended pieces per cached request
const CACHED_PIECES_RECOMMENDED_LIMIT: usize = 10;
```

📍 **Source**: [`shared/subspace-data-retrieval/src/object_fetcher.rs`](https://github.com/autonomys/subspace/blob/main/shared/subspace-data-retrieval/src/object_fetcher.rs)

## 11. Implementation Notes

### 11.1 Piece Masking

L1 pieces are masked during plotting:
1. Original piece data is encoded
2. Commitment is calculated
3. Data is XORed with farmer-specific mask
4. Stored in plot with metadata

### 11.2 Object Reconstruction

Objects spanning multiple pieces:
1. Determine required pieces from mapping
2. Fetch pieces (may span segments)
3. Handle segment headers and padding
4. Concatenate and verify hash

📍 **Implementation**: [`shared/subspace-data-retrieval/src/object_fetcher.rs`](https://github.com/autonomys/subspace/blob/main/shared/subspace-data-retrieval/src/object_fetcher.rs)

### 11.3 Segment Boundaries

Special handling required for:
- Objects crossing segment boundaries
- Parent segment headers at segment start
- Variable padding at segment end

## 12. Version History

- **v1.0**: Initial specification
- Future versions will maintain backwards compatibility 