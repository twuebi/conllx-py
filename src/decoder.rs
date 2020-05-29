use pyo3::prelude::*;

use crate::PySentence;
use std::ops::{Deref, DerefMut};
use sticker_encoders::deprel::{DependencyEncoding, RelativePOS, RelativePOSEncoder};
use sticker_encoders::{EncodingProb, SentenceDecoder};
use sticker_encoders::deprel::POSLayer;

/// Decodes RelativePOS labels
#[pyclass(name = Decoder)]
pub struct PyDecoder {
    inner: RelativePOSEncoder,
}

#[pymethods]
impl PyDecoder {
    #[new]
    pub fn new() -> Self {
        PyDecoder {
            inner: RelativePOSEncoder::new(POSLayer::Pos, "root"),
        }
    }

    /// Decode the Sentence - Label pairs
    fn decode_sentences(
        &self,
        mut sentences: Vec<PyRefMut<PySentence>>,
        labels: Vec<Vec<Vec<PyLabel>>>,
    ) -> PyResult<()> {
        let _ = sentences
            .iter_mut()
            .zip(labels.deref())
            .map(|(sent, sent_labels)| {
                let s = &mut **sent;
                let sent_labels: Vec<Vec<_>> = sent_labels
                    .iter()
                    .map(|tok| {
                        tok.iter()
                            .map(|single| single.into())
                            .collect::<Vec<EncodingProb<DependencyEncoding<RelativePOS>>>>()
                    })
                    .collect::<Vec<Vec<_>>>();
                self.inner.decode(&sent_labels, s.inner().deref_mut())
            })
            .collect::<Vec<_>>();
        Ok(())
    }
}

/// Quadruple containing DISTANCE/HEAD_POS/HEAD_RELATION/PROBABILITY
#[pyclass(name = Label)]
#[derive(Clone)]
pub struct PyLabel {
    distance: isize,
    pos: String,
    relation: String,
    probability: f32,
}

#[pymethods]
impl PyLabel {
    #[new]
    fn new(distance: isize, pos: &str, relation: &str, probability: f32) -> Self {
        PyLabel {
            distance,
            pos: pos.to_string(),
            relation: relation.to_string(),
            probability,
        }
    }
}

impl<'a> Into<EncodingProb<DependencyEncoding<RelativePOS>>> for &PyLabel {
    fn into(self) -> EncodingProb<DependencyEncoding<RelativePOS>> {
        let rel_pos = RelativePOS::new(self.pos.to_string(), self.distance);
        EncodingProb::new(
            DependencyEncoding::new(rel_pos, self.relation.to_string()),
            self.probability,
        )
    }
}
