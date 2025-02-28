use crate::plotting::{PieceGetter, PieceGetterRetryPolicy};
use futures::stream::FuturesOrdered;
use futures::StreamExt;
use std::sync::atomic::{AtomicUsize, Ordering};
use subspace_archiving::piece_reconstructor::{PiecesReconstructor, ReconstructorError};
use subspace_core_primitives::crypto::kzg::Kzg;
use subspace_core_primitives::{ArchivedHistorySegment, Piece, PieceIndex, RecordedHistorySegment};
use thiserror::Error;
use tokio::sync::Semaphore;
use tracing::{debug, error, info, trace, warn};

// TODO: Probably should be made configurable
const PARALLELISM_LEVEL: usize = 20;

#[derive(Debug, Error)]
pub(crate) enum SegmentReconstructionError {
    /// Not enough pieces to reconstruct a segment
    #[error("Not enough pieces to reconstruct a segment")]
    NotEnoughPiecesAcquired,

    /// Internal piece retrieval process failed
    #[error("Pieces retrieval failed")]
    PieceRetrievalFailed(#[from] ReconstructorError),
}

pub(crate) async fn recover_missing_piece<PG: PieceGetter>(
    piece_getter: &PG,
    kzg: Kzg,
    missing_piece_index: PieceIndex,
) -> Result<Piece, SegmentReconstructionError> {
    info!(%missing_piece_index, "Recovering missing piece...");
    let segment_index = missing_piece_index.segment_index();
    let position = missing_piece_index.position();

    let semaphore = Semaphore::new(PARALLELISM_LEVEL);
    let acquired_pieces_counter = AtomicUsize::default();
    let required_pieces_number = RecordedHistorySegment::NUM_RAW_RECORDS;

    // This is so we can move references into the future below
    let semaphore = &semaphore;
    let acquired_pieces_counter = &acquired_pieces_counter;

    let mut received_segment_pieces = segment_index
        .segment_piece_indexes_source_first()
        .map(|piece_index| async move {
            let _permit = match semaphore.acquire().await {
                Ok(permit) => permit,
                Err(error) => {
                    warn!(
                        %piece_index,
                        %error,
                        "Semaphore was closed, interrupting piece recover..."
                    );
                    return None;
                }
            };

            if acquired_pieces_counter.load(Ordering::SeqCst) >= required_pieces_number {
                trace!(%piece_index, "Skipped piece acquiring.");

                return None;
            }

            let piece = piece_getter
                .get_piece(piece_index, PieceGetterRetryPolicy::Limited(0))
                .await;

            match piece {
                Ok(piece) => {
                    if let Some(piece) = piece {
                        acquired_pieces_counter.fetch_add(1, Ordering::SeqCst);

                        Some((piece_index, piece))
                    } else {
                        None
                    }
                }
                Err(error) => {
                    debug!(?error, %piece_index, "Failed to get piece");
                    None
                }
            }
        })
        .collect::<FuturesOrdered<_>>();

    let mut segment_pieces = vec![None::<Piece>; ArchivedHistorySegment::NUM_PIECES];
    while let Some(maybe_received_piece) = received_segment_pieces.next().await {
        if let Some((piece_index, received_piece)) = maybe_received_piece {
            segment_pieces
                .get_mut(piece_index.position() as usize)
                .expect("Piece position is by definition within segment; qed")
                .replace(received_piece);
        }
    }

    if acquired_pieces_counter.load(Ordering::SeqCst) < required_pieces_number {
        error!(%missing_piece_index, "Recovering missing piece failed.");

        return Err(SegmentReconstructionError::NotEnoughPiecesAcquired);
    }

    let archiver = PiecesReconstructor::new(kzg).expect("Internal constructor call must succeed.");

    let result = archiver.reconstruct_piece(&segment_pieces, position as usize)?;

    info!(%missing_piece_index, "Recovering missing piece succeeded.");

    Ok(result)
}
