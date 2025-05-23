# DSN Algorithms Specification

## 1. Overview

This document specifies the core algorithms used in the Subspace DSN for piece distribution, cache management, and retrieval strategies.

## 2. Piece Distribution Algorithm

### 2.1 Proximity-Based Storage

The DSN uses XOR distance to determine which farmers store which pieces, ensuring uniform distribution.

📍 **Implementation**: [`crates/subspace-networking/src/utils/key_with_distance.rs`](https://github.com/autonomys/subspace/blob/main/crates/subspace-networking/src/utils/key_with_distance.rs)

#### 2.1.1 Distance Calculation

```rust
fn calculate_distance(peer_id: &[u8], piece_index: &[u8]) -> Distance {
    // XOR each byte of peer_id with piece_index multihash
    let mut distance = vec![0u8; peer_id.len()];
    for i in 0..peer_id.len() {
        distance[i] = peer_id[i] ^ piece_index[i];
    }
    Distance::from_bytes(distance)
}
```

#### 2.1.2 Piece Selection for L1 Storage

```rust
fn select_pieces_for_plot(
    peer_id: PeerId,
    plot_size: u64,
    total_history_size: u64,
) -> Vec<PieceIndex> {
    let mut selected_pieces = Vec::new();
    let piece_count = plot_size / PIECE_SIZE;
    
    // Deterministic selection based on peer ID
    let mut rng = ChaCha8Rng::from_seed(peer_id.to_bytes());
    
    // Select pieces uniformly from history
    for i in 0..piece_count {
        let piece_index = rng.gen_range(0..total_history_size);
        selected_pieces.push(PieceIndex(piece_index));
    }
    
    selected_pieces
}
```

📍 **Related**: [`crates/subspace-farmer/src/single_disk_farm/plotting.rs`](https://github.com/autonomys/subspace/blob/main/crates/subspace-farmer/src/single_disk_farm/plotting.rs)

### 2.2 Plot Expiration Algorithm

Ensures new history is uniformly distributed as the blockchain grows.

```rust
fn should_expire_piece(
    piece_index: PieceIndex,
    current_history_size: u64,
    plot_creation_history_size: u64,
) -> bool {
    // Gradually expire pieces as history doubles
    let history_growth = current_history_size / plot_creation_history_size;
    
    if history_growth <= 1 {
        return false;
    }
    
    // Probability of expiration increases with history growth
    let expiration_probability = 1.0 - (1.0 / history_growth as f64);
    
    // Deterministic decision based on piece index
    let hash = blake3::hash(&piece_index.to_bytes());
    let hash_value = u64::from_le_bytes(hash.as_bytes()[0..8].try_into().unwrap());
    let threshold = (u64::MAX as f64 * expiration_probability) as u64;
    
    hash_value < threshold
}
```

## 3. L2 Cache Algorithms

### 3.1 Cache Piece Selection

Determines which pieces a farmer should cache based on proximity.

```rust
fn select_pieces_for_cache(
    peer_id: PeerId,
    segment_pieces: Vec<PieceIndex>,
    cache_capacity: usize,
) -> Vec<PieceIndex> {
    // Calculate distance for each piece
    let mut pieces_with_distance: Vec<(PieceIndex, Distance)> = segment_pieces
        .into_iter()
        .map(|piece| {
            let distance = calculate_distance(
                &peer_id.to_bytes(),
                &piece.to_multihash().to_bytes()
            );
            (piece, distance)
        })
        .collect();
    
    // Sort by distance (closest first)
    pieces_with_distance.sort_by_key(|(_, distance)| *distance);
    
    // Select closest pieces up to capacity
    pieces_with_distance
        .into_iter()
        .take(cache_capacity)
        .map(|(piece, _)| piece)
        .collect()
}
```

📍 **Implementation**: [`crates/subspace-farmer/src/farmer_cache.rs:484`](https://github.com/autonomys/subspace/blob/main/crates/subspace-farmer/src/farmer_cache.rs#L484) (similar logic)

### 3.2 Cache Synchronization

Algorithm for keeping cache synchronized with new segments.

```rust
async fn synchronize_cache(
    cache_state: &mut CacheState,
    new_segment: SegmentHeader,
    peer_id: PeerId,
) {
    let segment_pieces = new_segment.piece_indices();
    
    // Determine which pieces to cache
    let pieces_to_cache = select_pieces_for_cache(
        peer_id,
        segment_pieces,
        cache_state.free_capacity()
    );
    
    // Fetch and store pieces
    for piece_index in pieces_to_cache {
        if let Some(piece) = fetch_piece_from_network(piece_index).await {
            cache_state.insert(piece_index, piece);
        }
    }
    
    // Evict if over capacity
    while cache_state.is_over_capacity() {
        cache_state.evict_lru();
    }
}
```

📍 **Implementation**: [`crates/subspace-farmer/src/farmer_cache.rs:683`](https://github.com/autonomys/subspace/blob/main/crates/subspace-farmer/src/farmer_cache.rs#L683) (`process_segment_header` method)

### 3.3 Cache Eviction Policy

LRU (Least Recently Used) with proximity weighting.

```rust
struct CacheEntry {
    piece_index: PieceIndex,
    last_accessed: Instant,
    access_count: u64,
    distance: Distance,
}

fn evict_piece(cache: &mut HashMap<PieceIndex, CacheEntry>) -> Option<PieceIndex> {
    // Calculate eviction score (lower = more likely to evict)
    let piece_to_evict = cache
        .iter()
        .map(|(index, entry)| {
            let age = entry.last_accessed.elapsed().as_secs() as f64;
            let popularity = (entry.access_count as f64).log2() + 1.0;
            let proximity = 1.0 / (entry.distance.as_u64() as f64 + 1.0);
            
            // Weighted score: recent, popular, and close pieces stay
            let score = (popularity * 2.0) + (1.0 / age) + (proximity * 3.0);
            
            (*index, score)
        })
        .min_by(|a, b| a.1.partial_cmp(&b.1).unwrap())
        .map(|(index, _)| index)?;
    
    cache.remove(&piece_to_evict);
    Some(piece_to_evict)
}
```

📍 **Related**: Cache eviction is implicitly handled in [`crates/subspace-farmer/src/farmer_cache/piece_cache_state.rs`](https://github.com/autonomys/subspace/blob/main/crates/subspace-farmer/src/farmer_cache/piece_cache_state.rs)

## 4. Retrieval Algorithms

### 4.1 Piece Discovery via Random Walk

Used when direct retrieval fails.

```rust
async fn get_piece_by_random_walk(
    node: &Node,
    piece_index: PieceIndex,
    max_rounds: usize,
) -> Option<Piece> {
    for round in 0..max_rounds {
        // Generate random peer ID for this round
        let random_key = PeerId::random();
        
        // Find closest peers to random key
        let closest_peers = node
            .get_closest_peers(random_key.into())
            .await
            .take(20)
            .collect::<Vec<_>>()
            .await;
        
        // Query each peer for the piece
        for peer_id in closest_peers {
            if let Ok(response) = node
                .send_request(peer_id, PieceByIndexRequest { piece_index })
                .await
            {
                if let Some(piece) = response.piece {
                    return Some(piece);
                }
            }
        }
    }
    
    None
}
```

📍 **Implementation**: [`crates/subspace-networking/src/utils/piece_provider.rs:287`](https://github.com/autonomys/subspace/blob/main/crates/subspace-networking/src/utils/piece_provider.rs#L287) (`get_piece_by_random_walking`)

### 4.2 Parallel Piece Retrieval

For fetching multiple pieces efficiently.

```rust
async fn download_pieces_parallel(
    piece_indices: Vec<PieceIndex>,
    node: &Node,
    max_concurrent: usize,
) -> Vec<(PieceIndex, Option<Piece>)> {
    let semaphore = Arc::new(Semaphore::new(max_concurrent));
    let mut tasks = Vec::new();
    
    for piece_index in piece_indices {
        let permit = semaphore.clone().acquire_owned().await;
        let node = node.clone();
        
        let task = tokio::spawn(async move {
            let _permit = permit;
            
            // Try L2 first
            if let Some(piece) = try_get_from_cache(&node, piece_index).await {
                return (piece_index, Some(piece));
            }
            
            // Fall back to L1
            if let Some(piece) = try_get_from_archival(&node, piece_index).await {
                return (piece_index, Some(piece));
            }
            
            (piece_index, None)
        });
        
        tasks.push(task);
    }
    
    // Collect results
    futures::future::join_all(tasks)
        .await
        .into_iter()
        .map(|result| result.unwrap())
        .collect()
}
```

📍 **Implementation**: [`shared/subspace-data-retrieval/src/piece_fetcher.rs`](https://github.com/autonomys/subspace/blob/main/shared/subspace-data-retrieval/src/piece_fetcher.rs) (`download_pieces` function)

## 5. Piece Validation Algorithm

### 5.1 KZG Commitment Verification

```rust
fn validate_piece(
    piece_index: PieceIndex,
    piece_data: &[u8],
    commitment: &Commitment,
) -> bool {
    // Verify piece size
    if piece_data.len() != PIECE_SIZE {
        return false;
    }
    
    // Calculate expected commitment
    let expected_commitment = calculate_kzg_commitment(piece_data);
    
    // Verify commitment matches
    expected_commitment == *commitment
}
```

📍 **Implementation**: [`crates/subspace-farmer/src/farmer_piece_getter/piece_validator.rs`](https://github.com/autonomys/subspace/blob/main/crates/subspace-farmer/src/farmer_piece_getter/piece_validator.rs)

## 6. Object Reconstruction Algorithm

### 6.1 Multi-Piece Object Assembly

```rust
async fn reconstruct_object(
    mapping: GlobalObject,
    piece_getter: &impl PieceGetter,
) -> Result<Vec<u8>, Error> {
    let mut object_data = Vec::new();
    let mut current_piece_index = mapping.piece_index;
    let mut current_offset = mapping.offset as usize;
    
    loop {
        // Fetch current piece
        let piece = piece_getter
            .get_piece(current_piece_index)
            .await?;
        
        // Extract data from current offset
        let piece_data = &piece.as_ref()[current_offset..];
        
        // Check if object continues in next piece
        let (data_in_piece, continues) = parse_object_data(piece_data)?;
        
        object_data.extend_from_slice(data_in_piece);
        
        if !continues {
            break;
        }
        
        // Move to next piece
        current_piece_index = current_piece_index.next_source_index();
        current_offset = 0;
    }
    
    // Verify object hash
    let actual_hash = blake3::hash(&object_data);
    if actual_hash != mapping.hash {
        return Err(Error::InvalidHash);
    }
    
    Ok(object_data)
}
```

📍 **Implementation**: [`shared/subspace-data-retrieval/src/object_fetcher.rs:336`](https://github.com/autonomys/subspace/blob/main/shared/subspace-data-retrieval/src/object_fetcher.rs#L336) (`fetch_object` method)

## 7. Performance Optimizations

### 7.1 Batch Processing

```rust
fn process_batch<T, F>(
    items: Vec<T>,
    batch_size: usize,
    processor: F,
) -> Vec<Result<T, Error>>
where
    F: Fn(Vec<T>) -> Vec<Result<T, Error>>,
{
    items
        .chunks(batch_size)
        .flat_map(|batch| processor(batch.to_vec()))
        .collect()
}
```

### 7.2 Connection Pooling

```rust
struct ConnectionPool {
    connections: HashMap<PeerId, Connection>,
    max_per_peer: usize,
}

impl ConnectionPool {
    async fn get_connection(&mut self, peer_id: &PeerId) -> Result<&mut Connection> {
        if !self.connections.contains_key(peer_id) {
            let conn = establish_connection(peer_id).await?;
            self.connections.insert(*peer_id, conn);
        }
        
        Ok(self.connections.get_mut(peer_id).unwrap())
    }
}
```

📍 **Related**: Connection management in [`crates/subspace-networking/src/node_runner.rs`](https://github.com/autonomys/subspace/blob/main/crates/subspace-networking/src/node_runner.rs)

## 8. Constants and Parameters

```rust
/// Piece size in bytes
const PIECE_SIZE: usize = 1_048_576;

/// Default cache sync batch size
const SYNC_BATCH_SIZE: usize = 256;

/// Maximum concurrent piece downloads
const MAX_CONCURRENT_DOWNLOADS: usize = 10;

/// Random walk rounds for piece discovery
const DEFAULT_RANDOM_WALK_ROUNDS: usize = 3;

/// LRU cache time window
const CACHE_TIME_WINDOW: Duration = Duration::from_secs(3600);
```

📍 **Constants defined in**:
- [`crates/subspace-farmer/src/farmer_cache.rs:44`](https://github.com/autonomys/subspace/blob/main/crates/subspace-farmer/src/farmer_cache.rs#L44) (SYNC_BATCH_SIZE)
- [`crates/subspace-networking/src/utils/piece_provider.rs`](https://github.com/autonomys/subspace/blob/main/crates/subspace-networking/src/utils/piece_provider.rs) (retrieval logic)

## 9. Algorithm Complexity

| Algorithm | Time Complexity | Space Complexity |
|-----------|----------------|------------------|
| Distance Calculation | O(n) | O(n) |
| Piece Selection | O(n log n) | O(n) |
| Cache Eviction | O(n) | O(1) |
| Random Walk | O(k * p) | O(p) |
| Object Reconstruction | O(m) | O(m) |

Where:
- n = number of pieces
- k = random walk rounds
- p = peers per round
- m = object size

## 10. Future Optimizations

1. **Predictive Caching**: Use ML to predict piece access patterns
2. **Adaptive Timeouts**: Adjust timeouts based on network conditions
3. **Smart Peer Selection**: Prefer peers with better latency/reliability
4. **Compression**: Compress pieces for network transfer
5. **Parallel Validation**: Validate multiple pieces concurrently 