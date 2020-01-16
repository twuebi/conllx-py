use pyo3::exceptions;
use pyo3::prelude::*;

use crate::{ListVec, PySentence};
use pyo3::types::PyAny;
use std::ops::Deref;
use sticker_encoders::deprel::{DependencyEncoding, RelativePOS, RelativePOSEncoder};
use sticker_encoders::{EncodingProb, SentenceDecoder};

/// Decodes RelativePOS labels
#[pyclass(name = Decoder)]
pub struct PyDecoder {
    inner: RelativePOSEncoder,
}

#[pymethods]
impl PyDecoder {
    #[new]
    pub fn new(obj: &PyRawObject) -> PyResult<()> {
        obj.init(PyDecoder {
            inner: RelativePOSEncoder,
        });
        Ok(())
    }

    /// Decode the Sentence - Label pairs
    fn decode_sentences(
        &self,
        sentences: Vec<&PySentence>,
        labels: ListVec<ListVec<ListVec<PyLabel>>>,
    ) -> PyResult<()> {
        // We need borrowed_sents to exist as long as sents, since the
        // lifetimes of sentences are bound to RefMut returned by
        // PySentence::inner.
        let mut borrowed_sents = sentences
            .into_iter()
            .map(|sent| sent.inner())
            .collect::<Vec<_>>();
        let _ = borrowed_sents
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
                self.inner.decode(&sent_labels, s)
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
    fn new(
        obj: &PyRawObject,
        distance: isize,
        pos: &str,
        relation: &str,
        probability: f32,
    ) -> PyResult<()> {
        obj.init(PyLabel {
            distance,
            pos: pos.to_string(),
            relation: relation.to_string(),
            probability,
        });
        Ok(())
    }
}

impl<'a> FromPyObject<'a> for PyLabel {
    fn extract(ob: &'a PyAny) -> PyResult<Self> {
        let py_label = ob
            .downcast_ref::<PyLabel>()
            .map_err(|_| exceptions::TypeError::py_err("argument of type 'list' expected"))?;
        Ok(py_label.to_owned())
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
