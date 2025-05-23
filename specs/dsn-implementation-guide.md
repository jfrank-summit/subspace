# DSN Implementation Guide

## 1. Overview

This guide provides practical instructions for implementing DSN components. Each section includes code examples, best practices, and common pitfalls.

## 2. Implementing a DSN Node

### 2.1 Basic Node Setup

```rust
use subspace_networking::{Node, Config, construct};

async fn create_node() -> Result<Node, Error> {
    let config = Config {
        listen_on: vec!["/ip4/0.0.0.0/tcp/30333".parse()?],
        allow_non_global_addresses_in_dht: false,
        initial_peers: vec![],
        reserved_peers: vec![],
        // ... other config
    };
    
    let (node, node_runner) = construct(config)?;
    
    // Run node in background
    tokio::spawn(async move {
        node_runner.run().await;
    });
    
    Ok(node)
}
```

📍 **Example**: [`crates/subspace-farmer/src/bin/subspace-farmer/commands/shared/network.rs:105`](https://github.com/autonomys/subspace/blob/main/crates/subspace-farmer/src/bin/subspace-farmer/commands/shared/network.rs#L105)

### 2.2 Implementing Request Handlers

```rust
use subspace_networking::protocols::request_response::handlers::{
    PieceByIndexRequest, PieceByIndexRequestHandler,
};

fn create_piece_handler(
    piece_provider: Arc<dyn PieceProvider>,
) -> PieceByIndexRequestHandler {
    PieceByIndexRequestHandler::create(move |peer_id, request| {
        let piece_provider = piece_provider.clone();
        
        async move {
            let PieceByIndexRequest { piece_index, cached_pieces } = request;
            
            // Get main piece
            let piece = piece_provider.get_piece(piece_index).await;
            
            // Get additional cached pieces
            let mut additional_pieces = Vec::new();
            for &index in cached_pieces.iter().take(10) {
                if let Some(p) = piece_provider.get_cached_piece(index).await {
                    additional_pieces.push(p);
                }
            }
            
            PieceByIndexResponse {
                piece,
                cached_pieces: additional_pieces,
            }
        }
    })
}
```

📍 **Implementation**: [`crates/subspace-farmer/src/bin/subspace-farmer/commands/shared/network.rs:161`](https://github.com/autonomys/subspace/blob/main/crates/subspace-farmer/src/bin/subspace-farmer/commands/shared/network.rs#L161)

## 3. Implementing a Farmer

### 3.1 Plot Management

```rust
struct Plot {
    file: File,
    piece_count: u64,
    farmer_id: FarmerId,
}

impl Plot {
    async fn create(path: &Path, size: u64) -> Result<Self, Error> {
        // Create plot file
        let file = OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .open(path)?;
        
        // Allocate space
        file.set_len(size)?;
        
        let piece_count = size / PIECE_SIZE as u64;
        let farmer_id = FarmerId::new();
        
        Ok(Self {
            file,
            piece_count,
            farmer_id,
        })
    }
    
    async fn write_piece(
        &mut self,
        piece_index: PieceIndex,
        piece_data: &[u8],
    ) -> Result<(), Error> {
        let offset = self.piece_offset(piece_index);
        self.file.seek(SeekFrom::Start(offset))?;
        self.file.write_all(piece_data)?;
        Ok(())
    }
    
    fn piece_offset(&self, piece_index: PieceIndex) -> u64 {
        // Map piece index to plot position
        let position = piece_index.0 % self.piece_count;
        position * PIECE_SIZE as u64
    }
}
```

📍 **Related**: [`crates/subspace-farmer/src/single_disk_farm/plot.rs`](https://github.com/autonomys/subspace/blob/main/crates/subspace-farmer/src/single_disk_farm/plot.rs)

### 3.2 L2 Cache Implementation

```rust
use subspace_farmer::{FarmerCache, PieceCache};

struct L2CacheImpl {
    cache_file: File,
    index: HashMap<PieceIndex, PieceCacheOffset>,
    capacity: u32,
    used: u32,
}

#[async_trait]
impl PieceCache for L2CacheImpl {
    async fn write_piece(
        &self,
        offset: PieceCacheOffset,
        piece_index: PieceIndex,
        piece: &Piece,
    ) -> Result<(), Error> {
        // Seek to offset
        let byte_offset = offset.0 as u64 * PIECE_SIZE as u64;
        self.cache_file.seek(SeekFrom::Start(byte_offset))?;
        
        // Write piece data
        self.cache_file.write_all(piece.as_ref())?;
        
        // Update index
        self.index.insert(piece_index, offset);
        
        Ok(())
    }
    
    async fn read_piece(
        &self,
        offset: PieceCacheOffset,
    ) -> Result<Option<Piece>, Error> {
        let byte_offset = offset.0 as u64 * PIECE_SIZE as u64;
        self.cache_file.seek(SeekFrom::Start(byte_offset))?;
        
        let mut piece_data = vec![0u8; PIECE_SIZE];
        self.cache_file.read_exact(&mut piece_data)?;
        
        Ok(Some(Piece::from(piece_data)))
    }
    
    fn max_num_elements(&self) -> u32 {
        self.capacity
    }
}
```

📍 **Reference Implementation**: [`crates/subspace-farmer/src/single_disk_farm/piece_cache.rs`](https://github.com/autonomys/subspace/blob/main/crates/subspace-farmer/src/single_disk_farm/piece_cache.rs)

### 3.3 Piece Selection

```rust
async fn select_pieces_for_cache(
    farmer_id: &FarmerId,
    new_segment: &SegmentHeader,
    cache_capacity: usize,
) -> Vec<PieceIndex> {
    let peer_id = farmer_id.to_peer_id();
    let mut distances = Vec::new();
    
    // Calculate distances for all pieces in segment
    for piece_index in new_segment.piece_indices() {
        let key = KeyWithDistance::new(
            peer_id,
            piece_index.to_multihash()
        );
        distances.push((piece_index, key.distance()));
    }
    
    // Sort by distance and take closest
    distances.sort_by_key(|(_, dist)| *dist);
    distances
        .into_iter()
        .take(cache_capacity)
        .map(|(idx, _)| idx)
        .collect()
}
```

📍 **Implementation**: [`crates/subspace-farmer/src/farmer_cache.rs:517`](https://github.com/autonomys/subspace/blob/main/crates/subspace-farmer/src/farmer_cache.rs#L517) (similar logic in `should_include_piece_in_cache`)

## 4. Implementing Piece Retrieval

### 4.1 PieceProvider Implementation

```rust
struct DsnPieceProvider {
    node: Node,
    farmer_cache: FarmerCache,
    semaphore: Arc<Semaphore>,
}

#[async_trait]
impl PieceGetter for DsnPieceProvider {
    async fn get_piece(&self, piece_index: PieceIndex) -> Option<Piece> {
        // Try L2 cache first
        if let Some(piece) = self.try_cache(piece_index).await {
            return Some(piece);
        }
        
        // Try L1 archival storage
        if let Some(piece) = self.try_archival(piece_index).await {
            return Some(piece);
        }
        
        None
    }
}

impl DsnPieceProvider {
    async fn try_cache(&self, piece_index: PieceIndex) -> Option<Piece> {
        let key = RecordKey::from(piece_index.to_multihash());
        
        // Get providers from DHT
        let providers = self.node
            .get_providers(key)
            .await
            .ok()?;
        
        // Try each provider
        for provider in providers {
            let request = CachedPieceByIndexRequest {
                piece_index,
                cached_pieces: Arc::new(vec![]),
            };
            
            match self.node.send_request(provider, request).await {
                Ok(CachedPieceByIndexResponse::Cached(result)) => {
                    return Some(result.piece);
                }
                _ => continue,
            }
        }
        
        None
    }
    
    async fn try_archival(&self, piece_index: PieceIndex) -> Option<Piece> {
        // Try connected peers first
        let connected = self.node.connected_peers().await.ok()?;
        
        for peer in connected {
            if let Some(piece) = self.get_from_peer(peer, piece_index).await {
                return Some(piece);
            }
        }
        
        // Fall back to random walk
        self.random_walk_search(piece_index, 3).await
    }
}
```

📍 **Example**: [`crates/subspace-farmer/src/farmer_piece_getter.rs`](https://github.com/autonomys/subspace/blob/main/crates/subspace-farmer/src/farmer_piece_getter.rs)

### 4.2 Batch Retrieval

```rust
async fn get_pieces_batch(
    provider: &DsnPieceProvider,
    indices: Vec<PieceIndex>,
) -> Vec<(PieceIndex, Option<Piece>)> {
    let mut futures = FuturesUnordered::new();
    
    for piece_index in indices {
        let provider = provider.clone();
        futures.push(async move {
            let piece = provider.get_piece(piece_index).await;
            (piece_index, piece)
        });
    }
    
    let mut results = Vec::new();
    while let Some(result) = futures.next().await {
        results.push(result);
    }
    
    results
}
```

📍 **Implementation**: [`shared/subspace-data-retrieval/src/piece_fetcher.rs:96`](https://github.com/autonomys/subspace/blob/main/shared/subspace-data-retrieval/src/piece_fetcher.rs#L96) (`download_pieces`)

## 5. Implementing Object Fetching

### 5.1 Basic Object Fetcher

```rust
struct ObjectFetcherImpl<PG: PieceGetter> {
    piece_getter: Arc<PG>,
    max_object_size: usize,
}

impl<PG: PieceGetter> ObjectFetcherImpl<PG> {
    async fn fetch_object(
        &self,
        mapping: GlobalObject,
    ) -> Result<Vec<u8>, Error> {
        let mut data = Vec::new();
        let mut current_piece = mapping.piece_index;
        let mut offset = mapping.offset as usize;
        
        loop {
            // Get piece
            let piece = self.piece_getter
                .get_piece(current_piece)
                .await
                .ok_or(Error::PieceNotFound)?;
            
            // Read data from offset
            let piece_data = &piece.as_ref()[offset..];
            
            // Parse length prefix if first piece
            let (object_data, continues) = if data.is_empty() {
                self.parse_with_length(piece_data)?
            } else {
                self.parse_continuation(piece_data)?
            };
            
            data.extend_from_slice(object_data);
            
            if !continues || data.len() >= self.max_object_size {
                break;
            }
            
            // Move to next piece
            current_piece = current_piece.next_source_index();
            offset = 0;
        }
        
        // Verify hash
        let hash = blake3::hash(&data);
        if hash != mapping.hash {
            return Err(Error::InvalidHash);
        }
        
        Ok(data)
    }
}
```

📍 **Implementation**: [`shared/subspace-data-retrieval/src/object_fetcher.rs`](https://github.com/autonomys/subspace/blob/main/shared/subspace-data-retrieval/src/object_fetcher.rs)

## 6. Best Practices

### 6.1 Error Handling

```rust
#[derive(Debug, thiserror::Error)]
enum DsnError {
    #[error("Piece not found: {0:?}")]
    PieceNotFound(PieceIndex),
    
    #[error("Network error: {0}")]
    Network(#[from] NetworkError),
    
    #[error("Storage error: {0}")]
    Storage(#[from] io::Error),
}

// Use Result type alias
type Result<T> = std::result::Result<T, DsnError>;
```

### 6.2 Metrics Collection

```rust
use prometheus_client::{Counter, Gauge};

struct DsnMetrics {
    pieces_retrieved: Counter,
    cache_hits: Counter,
    cache_misses: Counter,
    retrieval_time: Histogram,
}

impl DsnMetrics {
    fn record_retrieval(&self, from_cache: bool, duration: Duration) {
        self.pieces_retrieved.inc();
        
        if from_cache {
            self.cache_hits.inc();
        } else {
            self.cache_misses.inc();
        }
        
        self.retrieval_time.observe(duration.as_secs_f64());
    }
}
```

📍 **Example**: [`crates/subspace-farmer/src/farmer_cache/metrics.rs`](https://github.com/autonomys/subspace/blob/main/crates/subspace-farmer/src/farmer_cache/metrics.rs)

### 6.3 Testing

```rust
#[cfg(test)]
mod tests {
    use super::*;
    
    #[tokio::test]
    async fn test_piece_retrieval() {
        // Create mock piece getter
        let mut mock = MockPieceGetter::new();
        mock.expect_get_piece()
            .with(eq(PieceIndex(42)))
            .returning(|_| Some(Piece::default()));
        
        // Test retrieval
        let provider = DsnPieceProvider::new(mock);
        let piece = provider.get_piece(PieceIndex(42)).await;
        
        assert!(piece.is_some());
    }
}
```

## 7. Common Pitfalls

### 7.1 Resource Leaks

```rust
// BAD: Connection leak
async fn get_many_pieces(indices: Vec<PieceIndex>) {
    for index in indices {
        let conn = connect_to_peer().await; // New connection each time!
        get_piece_from_peer(conn, index).await;
        // Connection not properly closed
    }
}

// GOOD: Connection reuse
async fn get_many_pieces(indices: Vec<PieceIndex>) {
    let conn = connect_to_peer().await;
    
    for index in indices {
        get_piece_from_peer(&conn, index).await;
    }
    
    conn.close().await;
}
```

### 7.2 Unbounded Concurrency

```rust
// BAD: Can overwhelm system
async fn download_all(indices: Vec<PieceIndex>) {
    let futures: Vec<_> = indices
        .into_iter()
        .map(|idx| download_piece(idx))
        .collect();
    
    futures::future::join_all(futures).await;
}

// GOOD: Bounded concurrency
async fn download_all(indices: Vec<PieceIndex>) {
    let semaphore = Arc::new(Semaphore::new(10));
    let mut handles = vec![];
    
    for idx in indices {
        let permit = semaphore.clone().acquire_owned().await.unwrap();
        handles.push(tokio::spawn(async move {
            let _permit = permit;
            download_piece(idx).await
        }));
    }
    
    futures::future::join_all(handles).await;
}
```

📍 **Example**: [`shared/subspace-data-retrieval/src/piece_fetcher.rs:96`](https://github.com/autonomys/subspace/blob/main/shared/subspace-data-retrieval/src/piece_fetcher.rs#L96)

## 8. Integration Examples

### 8.1 Complete Farmer Setup

```rust
async fn setup_farmer(
    base_path: PathBuf,
    plot_size: u64,
    cache_size: u64,
) -> Result<Farmer> {
    // Create directories
    fs::create_dir_all(&base_path)?;
    
    // Initialize plot
    let plot_path = base_path.join("plot.bin");
    let plot = Plot::create(&plot_path, plot_size).await?;
    
    // Initialize cache
    let cache_path = base_path.join("cache.bin");
    let cache = L2CacheImpl::create(&cache_path, cache_size).await?;
    
    // Create networking node
    let node = create_node().await?;
    
    // Create farmer cache
    let (farmer_cache, cache_worker) = FarmerCache::new(
        node.clone(),
        plot.farmer_id.to_peer_id(),
        None, // No metrics registry
    );
    
    // Start cache worker
    tokio::spawn(async move {
        cache_worker.run(piece_getter).await;
    });
    
    Ok(Farmer {
        plot,
        cache,
        farmer_cache,
        node,
    })
}
```

📍 **Reference**: [`crates/subspace-farmer/src/single_disk_farm.rs`](https://github.com/autonomys/subspace/blob/main/crates/subspace-farmer/src/single_disk_farm.rs)

## 9. Performance Tips

1. **Batch Operations**: Always batch piece requests when possible
2. **Connection Pooling**: Reuse connections to peers
3. **Concurrent Limits**: Use semaphores to limit concurrency
4. **Caching**: Cache frequently accessed pieces locally
5. **Metrics**: Monitor performance to identify bottlenecks

## 10. Debugging

### 10.1 Enable Logging

```rust
use tracing::{info, debug, warn, error};

fn init_logging() {
    tracing_subscriber::fmt()
        .with_env_filter("subspace=debug,libp2p=info")
        .init();
}
```

### 10.2 Common Issues

- **Piece Not Found**: Check if piece index is valid and if farmers are online
- **Slow Retrieval**: Check network latency and cache hit rates
- **High CPU**: Look for unbounded loops or excessive polling
- **Memory Leaks**: Check for accumulated futures or unclosed connections 