use std::cell::{Ref, RefCell, RefMut};
use std::rc::Rc;

use conllx::graph::{Node, Sentence};
use conllx::token::{Features, Token, TokenBuilder};
use pyo3::class::basic::PyObjectProtocol;
use pyo3::class::iter::PyIterProtocol;
use pyo3::class::mapping::PyMappingProtocol;
use pyo3::class::sequence::PySequenceProtocol;
use pyo3::exceptions;
use pyo3::prelude::*;

use crate::lemma::PyEditTree;
use crate::util::ListVec;

/// Sentence that can be annotated.
#[pyclass(name = Sentence)]
pub struct PySentence {
    inner: Rc<RefCell<Sentence>>,
}

#[pymethods]
impl PySentence {
    /// Construct a new sentence from forms and (optionally) POS tags.
    ///
    /// The constructor will throw a `ValueError` if POS tags are
    /// provided, but the number or tags is not equal to the number of
    /// tokens.
    #[new]
    fn __new__(
        obj: &PyRawObject,
        forms: ListVec<&str>,
        pos_tags: Option<ListVec<&str>>,
        lemmas: Option<ListVec<&str>>,
    ) -> PyResult<()> {
        let sent = match (pos_tags, lemmas) {
            (Some(pos_tags), Some(lemmas)) => {
                if forms.len() != pos_tags.len() {
                    return Err(exceptions::ValueError::py_err(format!(
                        "token and part-of-speech lists have inequal lengths: {} {}",
                        forms.len(),
                        pos_tags.len()
                    )));
                }
                if forms.len() != lemmas.len() {
                    return Err(exceptions::ValueError::py_err(format!(
                        "token and lemma lists have inequal lengths: {} {}",
                        forms.len(),
                        lemmas.len()
                    )));
                }
                forms
                    .into_inner()
                    .into_iter()
                    .zip(pos_tags.into_inner())
                    .zip(lemmas.into_inner())
                    .map(|((form, pos_tag), lemma)| {
                        TokenBuilder::new(form).pos(pos_tag).lemma(lemma).into()
                    })
                    .collect::<Sentence>()
            }
            (None, Some(lemmas)) => {
                if forms.len() != lemmas.len() {
                    return Err(exceptions::ValueError::py_err(format!(
                        "token and lemma lists have inequal lengths: {} {}",
                        forms.len(),
                        lemmas.len()
                    )));
                }
                forms
                    .into_inner()
                    .into_iter()
                    .zip(lemmas.into_inner())
                    .map(|(form, lemma)| TokenBuilder::new(form).lemma(lemma).into())
                    .collect::<Sentence>()
            }
            (Some(pos_tags), None) => {
                if forms.len() != pos_tags.len() {
                    return Err(exceptions::ValueError::py_err(format!(
                        "token and part-of-speech lists have inequal lengths: {} {}",
                        forms.len(),
                        pos_tags.len()
                    )));
                }
                forms
                    .into_inner()
                    .into_iter()
                    .zip(pos_tags.into_inner())
                    .map(|(form, pos_tag)| TokenBuilder::new(form).pos(pos_tag).into())
                    .collect::<Sentence>()
            }
            (None, None) => forms
                .into_inner()
                .into_iter()
                .map(Token::new)
                .collect::<Sentence>(),
        };

        obj.init(PySentence {
            inner: Rc::new(sent.into()),
        });

        Ok(())
    }
}

impl PySentence {
    pub fn new(sent: Sentence) -> Self {
        PySentence {
            inner: Rc::new(sent.into()),
        }
    }
    pub fn inner(&self) -> RefMut<Sentence> {
        self.inner.borrow_mut()
    }
}

#[pyproto]
impl PyIterProtocol for PySentence {
    fn __iter__(slf: PyRefMut<Self>) -> PyResult<PySentenceIterator> {
        Ok(PySentenceIterator {
            sent: slf.inner.clone(),
            idx: 0,
        })
    }
}

#[pyproto]
impl PyObjectProtocol for PySentence {
    fn __repr__(&self) -> PyResult<String> {
        let token_reprs = (0..self.inner.borrow().len())
            .map(|token_idx| {
                PyToken {
                    token_idx,
                    sent: self.inner.clone(),
                }
                .__repr__()
            })
            .collect::<Result<Vec<_>, _>>()?;

        Ok(format!("Sentence([{}])", token_reprs.join(", ")))
    }

    fn __str__(&self) -> PyResult<String> {
        Ok(format!("{}", self.inner.borrow()))
    }
}

#[pyproto]
impl PySequenceProtocol for PySentence {
    fn __len__(&self) -> PyResult<usize> {
        Ok(self.inner.borrow().len())
    }

    fn __getitem__(&self, idx: isize) -> PyResult<PyToken> {
        if idx >= self.inner.borrow().len() as isize || idx < 0 {
            Err(exceptions::IndexError::py_err("token index out of range"))
        } else {
            Ok(PyToken {
                sent: self.inner.clone(),
                token_idx: idx as usize,
            })
        }
    }
}

/// Iterator over the nodes in a dependency graph.
///
/// The nodes are returned in sentence-linear order.
#[pyclass(name = SentenceIterator)]
pub struct PySentenceIterator {
    sent: Rc<RefCell<Sentence>>,
    idx: usize,
}

#[pyproto]
impl PyIterProtocol for PySentenceIterator {
    fn __iter__(slf: PyRefMut<Self>) -> PyResult<Py<PySentenceIterator>> {
        Ok(slf.into())
    }

    fn __next__(mut slf: PyRefMut<Self>) -> PyResult<Option<PyToken>> {
        let slf = &mut *slf;

        if slf.idx < slf.sent.borrow().len() {
            let token = PyToken {
                sent: slf.sent.clone(),
                token_idx: slf.idx,
            };

            slf.idx += 1;

            Ok(Some(token))
        } else {
            Ok(None)
        }
    }
}

/// Token that can be annotated.
#[pyclass(name = Token)]
pub struct PyToken {
    sent: Rc<RefCell<Sentence>>,
    token_idx: usize,
}

#[pymethods]
impl PyToken {
    /// Get the features of a token.
    #[getter]
    fn get_features(&self) -> PyFeatures {
        PyFeatures {
            sent: self.sent.clone(),
            token_idx: self.token_idx,
        }
    }

    /// Get the form of the token.
    #[getter]
    fn get_form(&self) -> Option<String> {
        match self.sent.borrow()[self.token_idx] {
            Node::Token(ref token) => Some(token.form().to_owned()),
            Node::Root => None,
        }
    }

    /// Get the form of the token.
    #[getter]
    fn get_lemma(&self) -> Option<String> {
        match self.sent.borrow()[self.token_idx] {
            Node::Token(ref token) => token.lemma().map(ToOwned::to_owned),
            Node::Root => None,
        }
    }

    /// Get the (dependency) head.
    #[getter]
    fn get_head(&self) -> Option<usize> {
        self.sent
            .borrow()
            .dep_graph()
            .head(self.token_idx)
            .map(|triple| triple.head())
    }

    /// Get the (dependency) head relation.
    #[getter]
    fn get_head_rel(&self) -> Option<String> {
        self.sent
            .borrow()
            .dep_graph()
            .head(self.token_idx)
            .and_then(|triple| triple.relation().map(ToOwned::to_owned))
    }

    /// Get the part-of-speech.
    #[getter]
    fn get_pos(&self) -> Option<String> {
        match self.sent.borrow()[self.token_idx] {
            Node::Token(ref token) => token.pos().map(ToOwned::to_owned),
            Node::Root => None,
        }
    }

    #[setter]
    fn set_pos(&mut self, pos: &str) -> PyResult<()> {
        match self.sent.borrow_mut()[self.token_idx].token_mut() {
            Some(token) => {
                token.set_pos(Some(pos));
                Ok(())
            }
            None => Err(exceptions::Exception::py_err("cannot set pos of root")),
        }
    }

    #[setter]
    fn set_lemma(&mut self, lemma: &str) -> PyResult<()> {
        match self.sent.borrow_mut()[self.token_idx].token_mut() {
            Some(token) => {
                token.set_lemma(Some(lemma));
                Ok(())
            }
            None=> Err(exceptions::Exception::py_err("cannot set pos of root")),
        }
    }

    fn compute_edit_tree(&self) -> PyResult<PyEditTree> {
        match self.sent.borrow()[self.token_idx] {
            Node::Token(ref token) => {
                let lemma = match token.lemma() {
                    Some(lemma) => lemma,
                    None => token.form(),
                };
                Ok(PyEditTree::new(token.form().chars(), lemma.chars()))
            }
            Node::Root => Err(exceptions::Exception::py_err(
                "cannot compute edit tree for root",
            )),
        }
    }
}

#[pyproto]
impl PyObjectProtocol for PyToken {
    fn __repr__(&self) -> PyResult<String> {
        match self.sent.borrow()[self.token_idx] {
            Node::Root => Ok("Root".to_string()),
            Node::Token(ref token) => {
                let mut attrs = Vec::new();
                attrs.push(format!("form = '{}'", token.form()));
                if let Some(lemma) = token.lemma() {
                    attrs.push(format!("lemma = '{}'", lemma));
                }
                if let Some(pos) = token.pos() {
                    attrs.push(format!("pos = '{}'", pos));
                };

                if let Some(triple) = self.sent.borrow().dep_graph().head(self.token_idx) {
                    attrs.push(format!("head = {}", triple.head()));

                    if let Some(relation) = triple.relation() {
                        attrs.push(format!("relation = {}", relation));
                    }
                }

                Ok(format!("Token({})", attrs.join(", ")))
            }
        }
    }
}

/// Token that can be annotated.
#[pyclass(name = Features)]
pub struct PyFeatures {
    sent: Rc<RefCell<Sentence>>,
    token_idx: usize,
}

#[pymethods]
impl PyFeatures {
    fn get(&self, name: &str, default: Option<&str>) -> PyResult<Option<String>> {
        let token = self.token()?;

        match token
            .features()
            .and_then(|features| features.get(name))
            // On the Rust side, we can have features without values.
            // Not sure if/how we want to handle this in Python.
            .and_then(|feature| feature.as_ref())
            .map(ToOwned::to_owned)
        {
            Some(val) => Ok(Some(val)),
            None => Ok(default.map(ToOwned::to_owned)),
        }
    }

    fn contains(&self, name: &str) -> PyResult<bool> {
        let token = self.token()?;

        Ok(token
            .features()
            .and_then(|features| features.get(name))
            // On the Rust side, we can have features without values.
            // Not sure if/how we want to handle this in Python.
            .and_then(|feature| feature.as_ref())
            .is_some())
    }
}

impl PyFeatures {
    fn token(&self) -> PyResult<Ref<Token>> {
        let sent = self.sent.borrow();

        if sent[self.token_idx].is_root() {
            return Err(exceptions::KeyError::py_err(
                "root node does not have features",
            ));
        }

        let token = Ref::map(sent, |sent| sent[self.token_idx].token().unwrap());

        Ok(token)
    }

    fn token_mut(&mut self) -> PyResult<RefMut<Token>> {
        let sent = self.sent.borrow_mut();

        if sent[self.token_idx].is_root() {
            return Err(exceptions::KeyError::py_err(
                "root node does not have features",
            ));
        }

        let token = RefMut::map(sent, |sent| sent[self.token_idx].token_mut().unwrap());

        Ok(token)
    }
}

#[pyproto]
impl PyMappingProtocol for PyFeatures {
    fn __delitem__(&mut self, name: &str) -> PyResult<()> {
        let mut token = self.token_mut()?;

        let _ = token
            .features_mut()
            .and_then(|features| features.remove(name))
            .ok_or(exceptions::KeyError::py_err(format!(
                "features set does not contain feature: {}",
                name
            )))?;

        Ok(())
    }

    fn __getitem__(&self, name: &str) -> PyResult<String> {
        let token = self.token()?;

        token
            .features()
            .and_then(|features| features.get(name))
            // On the Rust side, we can have features without values.
            // Not sure if/how we want to handle this in Python.
            .and_then(|feature| feature.as_ref())
            .map(ToOwned::to_owned)
            .ok_or(exceptions::KeyError::py_err(format!(
                "unknown feature: {}",
                name
            )))
    }

    fn __setitem__(&mut self, name: String, value: String) -> PyResult<()> {
        let mut token = self.token_mut()?;

        if token.features().is_none() {
            token.set_features(Some(Features::default()));
        }

        token
            .features_mut()
            .unwrap()
            .insert(name.to_owned(), Some(value.to_owned()));

        Ok(())
    }
}

#[pyproto]
impl PyObjectProtocol for PyFeatures {
    fn __repr__(&self) -> PyResult<String> {
        let dict_repr = match &self.sent.borrow()[self.token_idx] {
            Node::Root => String::new(),
            Node::Token(token) => match token.features() {
                Some(features) => {
                    let fvals = features
                        .iter()
                        // Filter out features without values.
                        .filter_map(|(f, v)| v.as_ref().map(|v| (f, v)))
                        .map(|(f, v)| format!("\"{}\": \"{}\"", f, v))
                        .collect::<Vec<_>>();
                    fvals.join(", ")
                }
                None => String::new(),
            },
        };

        Ok(format!("Features {{{}}}", dict_repr))
    }

    fn __str__(&self) -> PyResult<String> {
        self.__repr__()
    }
}
