# DSN Specification Documents

This folder contains the specification and documentation for the Subspace Distributed Storage Network (DSN).

## 📄 Start Here

### [DSN Specification Summary](./dsn-specification-summary.md)
A concise overview of the entire DSN system, perfect for getting started.

## 📍 Code Navigation

All specification documents include direct links to the actual implementation code. Look for the 📍 **Source** or 📍 **Implementation** markers throughout the documents to jump directly to the relevant code.

## Documents

### Core Specifications

#### 1. [DSN Specification Outline](./dsn-specification-outline.md)
A comprehensive outline for the full DSN specification, organized by topic. This serves as the structure for developing the complete specification.

#### 2. [DSN Protocols Specification](./dsn-protocols-specification.md)
Detailed specification of the DSN networking protocols including:
- Message formats and SCALE encoding
- Protocol flows and sequence diagrams
- Error handling and security considerations
- Performance guidelines and metrics

#### 3. [DSN Data Structures](./dsn-data-structures.md)
Formal specification of all data structures including:
- Basic types (PieceIndex, SegmentIndex, etc.)
- Piece and segment structures
- Object mapping structures
- Networking and cache structures
- SCALE encoding details

#### 4. [DSN Algorithms](./dsn-algorithms.md)
Detailed algorithms specification covering:
- Piece distribution and proximity calculations
- Cache management and eviction policies
- Retrieval strategies and random walk
- Object reconstruction
- Performance optimizations

### Implementation Documentation

#### 5. [DSN Implementation Guide](./dsn-implementation-guide.md)
Practical guide for implementing DSN components:
- Node setup and configuration
- Farmer implementation
- Piece retrieval strategies
- Object fetching
- Best practices and common pitfalls

#### 6. [DSN Implementation Mapping](./dsn-implementation-mapping.md)
Maps the conceptual DSN architecture to the actual implementation in the codebase. Shows which code components implement each layer and concept.

#### 7. [DSN Key Findings](./dsn-key-findings.md)
Summary of findings from analyzing the codebase, including:
- Current implementation status
- Design patterns
- Configuration parameters
- Performance characteristics

#### 8. [DSN Usage Patterns](./dsn-usage-patterns.md)
Comprehensive documentation of how DSN is used by various services:
- Node synchronization (sync from DSN, snap sync)
- Farmer operations (caching, plotting)
- Gateway service for object retrieval
- RPC endpoints and archiver integration
- Performance considerations and best practices

## Quick Reference

### DSN Layers
- **L1 (Archival Storage)**: Permanent storage in farmer plots (~1s retrieval)
- **L2 (Pieces Cache)**: Fast DHT-based cache (~10-100ms retrieval)

### Key Components
- `subspace-networking`: Core networking and protocols
- `subspace-farmer`: L2 cache implementation
- `subspace-data-retrieval`: High-level data APIs

### Main Protocols
- `PieceByIndexRequest`: L1 retrieval
- `CachedPieceByIndexRequest`: L2 retrieval
- `SegmentHeaderRequest`: Metadata retrieval

## Architecture Overview

The DSN implements a two-layer storage system:

1. **L1 - Archival Storage**: Farmers store encoded pieces in plots, providing permanent storage with ~1 second retrieval time
2. **L2 - Cache Layer**: A DHT-based cache storing unencoded pieces for fast retrieval (10-100ms)

Pieces are distributed based on proximity calculations, ensuring uniform distribution and efficient discovery through Kademlia DHT.

## Progress Summary

### ✅ Completed Specification Work
1. Created comprehensive DSN specification outline
2. Documented current implementation mapping
3. Analyzed codebase and documented key findings
4. Specified detailed protocol messages and flows
5. Created formal data structures specification
6. Documented core algorithms (distribution, caching, retrieval)
7. Developed practical implementation guide
8. Created executive summary document
9. **Added direct code links throughout all documents**
10. **Documented how DSN is used by various services**

### 🚧 Future Documentation Opportunities
1. Performance benchmarking guidelines
2. Security threat model documentation
3. Integration testing guide
4. Troubleshooting documentation
5. Migration guides for protocol updates

## Key Concepts

### Piece Distribution
- Uses XOR distance for proximity calculations
- Ensures uniform distribution across farmers
- Gradual plot expiration as history grows

### Cache Management
- Proximity-based piece selection
- LRU eviction with weighting
- Automatic synchronization with new segments

### Retrieval Strategy
1. Try L2 cache (fast)
2. Try connected L1 peers
3. Use random walk discovery
4. Validate retrieved pieces

## For Developers

### Getting Started
1. Start with the [Summary](./dsn-specification-summary.md) for an overview
2. Read the [Implementation Guide](./dsn-implementation-guide.md) for practical examples
3. Review [Data Structures](./dsn-data-structures.md) for type definitions
4. Study [Algorithms](./dsn-algorithms.md) for core logic
5. Check [Protocols](./dsn-protocols-specification.md) for network communication
6. Explore [Usage Patterns](./dsn-usage-patterns.md) to see how services use DSN

### Common Tasks
- **Implementing a Farmer**: See section 3 in Implementation Guide
- **Piece Retrieval**: See section 4 in Implementation Guide
- **Object Fetching**: See section 5 in Implementation Guide
- **Service Integration**: See Usage Patterns document
- **Debugging**: See section 10 in Implementation Guide

## Contributing

When adding new specifications:
1. Follow the existing document structure
2. Include code examples where helpful
3. Add complexity analysis for algorithms
4. Cross-reference related documents
5. Update this README with new documents
6. Consider implementation feasibility
7. **Include direct links to relevant code with 📍 markers**

## Document Map

```
DSN Specifications/
├── README.md (this file)
├── dsn-specification-summary.md     [START HERE]
├── dsn-specification-outline.md     [Overall structure]
├── dsn-protocols-specification.md   [Network protocols]
├── dsn-data-structures.md          [Type definitions]
├── dsn-algorithms.md               [Core algorithms]
├── dsn-implementation-guide.md     [How to implement]
├── dsn-implementation-mapping.md   [Code locations]
├── dsn-key-findings.md            [Analysis results]
└── dsn-usage-patterns.md          [Service integration]
``` 