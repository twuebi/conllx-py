use std::io::BufReader;

use crate::util::RandomRemoveVec;
use crate::PySentence;
use conllx::graph::Sentence;
use conllx::io::{ReadSentence, Reader};
use failure::Fallible;
use pyo3::class::iter::PyIterProtocol;
use pyo3::exceptions;
use pyo3::prelude::*;
use rand::SeedableRng;
use rand_xorshift::XorShiftRng;
use std::fs::File;

/// Iterator over the nodes in a dependency graph.
///
/// The nodes are returned in sentence-linear order.
#[pyclass(name = DataIterator)]
pub struct PyDataIterator {
    dataset: Box<dyn Iterator<Item = Fallible<Sentence>>>,
}

#[pyproto]
impl PyIterProtocol for PyDataIterator {
    fn __iter__(slf: PyRefMut<Self>) -> PyResult<Py<PyDataIterator>> {
        Ok(slf.into())
    }

    fn __next__(mut slf: PyRefMut<Self>) -> PyResult<Option<PySentence>> {
        match slf.dataset.next() {
            Some(sent) => match sent {
                Ok(sent) => Ok(Some(PySentence::new(sent))),
                _ => Err(exceptions::Exception::py_err("reading failed")),
            },
            None => Ok(None),
        }
    }
}

#[pymethods]
impl PyDataIterator {
    /// Construct a new sentence from forms and (optionally) POS tags.
    ///
    /// The constructor will throw a `ValueError` if POS tags are
    /// provided, but the number or tags is not equal to the number of
    /// tokens.
    #[new]
    fn __new__(
        obj: &PyRawObject,
        path: &str,
        max_len: Option<usize>,
        shuffle_buffer_size: Option<usize>,
    ) -> PyResult<()> {
        obj.init(PyDataIterator {
            dataset: ConllxDataSet::get_sentence_iter(
                path.to_string(),
                max_len,
                shuffle_buffer_size,
            ),
        });

        Ok(())
    }
}

/// A CoNLL-X data set.
pub struct ConllxDataSet(Reader<BufReader<File>>);

impl ConllxDataSet {
    /// Returns an `Iterator` over `Result<Sentence, Error>`.
    ///
    /// Depending on the parameters the returned iterator filters
    /// sentences by their lengths or returns the sentences in
    /// sequence without filtering them.
    ///
    /// If `max_len` == `None`, no filtering is performed.
    fn get_sentence_iter<'ds, 'a: 'ds>(
        reader: String,
        max_len: Option<usize>,
        shuffle_buffer_size: Option<usize>,
    ) -> Box<dyn Iterator<Item = Fallible<Sentence>> + 'ds>
where {
        let f = File::open(reader).unwrap();
        let f = BufReader::new(f);
        let r = Reader::new(f);
        let tokenized_sentences = r.sentences();

        match (max_len, shuffle_buffer_size) {
            (Some(max_len), Some(buffer_size)) => Box::new(
                tokenized_sentences
                    .filter_by_len(max_len)
                    .shuffle(buffer_size),
            ),
            (Some(max_len), None) => Box::new(tokenized_sentences.filter_by_len(max_len)),
            (None, Some(buffer_size)) => Box::new(tokenized_sentences.shuffle(buffer_size)),
            (None, None) => Box::new(tokenized_sentences),
        }
    }
}

/// Trait providing adapters for `SentenceWithPieces` iterators.
pub trait SentenceIter: Sized {
    fn filter_by_len(self, max_len: usize) -> LengthFilter<Self>;
    fn shuffle(self, buffer_size: usize) -> Shuffled<Self>;
}

impl<I> SentenceIter for I
where
    I: Iterator<Item = Fallible<Sentence>>,
{
    fn filter_by_len(self, max_len: usize) -> LengthFilter<Self> {
        LengthFilter {
            inner: self,
            max_len,
        }
    }

    fn shuffle(self, buffer_size: usize) -> Shuffled<Self> {
        Shuffled {
            inner: self,
            buffer: RandomRemoveVec::with_capacity(buffer_size, XorShiftRng::from_entropy()),
            buffer_size,
        }
    }
}

/// An Iterator adapter filtering sentences by maximum length.
pub struct LengthFilter<I> {
    inner: I,
    max_len: usize,
}

impl<I> Iterator for LengthFilter<I>
where
    I: Iterator<Item = Fallible<Sentence>>,
{
    type Item = Fallible<Sentence>;

    fn next(&mut self) -> Option<Self::Item> {
        while let Some(sent) = self.inner.next() {
            // Treat Err as length 0 to keep our type as Result<Sentence, Error>. The iterator
            // will properly return the Error at a later point.
            let len = sent.as_ref().map(|s| s.len()).unwrap_or(0);
            if len > self.max_len {
                continue;
            }
            return Some(sent);
        }
        None
    }
}

/// An Iterator adapter performing local shuffling.
///
/// Fills a buffer with size `buffer_size` on the first
/// call. Subsequent calls add the next incoming item to the buffer
/// and pick a random element from the buffer.
pub struct Shuffled<I> {
    inner: I,
    buffer: RandomRemoveVec<Sentence, XorShiftRng>,
    buffer_size: usize,
}

impl<I> Iterator for Shuffled<I>
where
    I: Iterator<Item = Fallible<Sentence>>,
{
    type Item = Fallible<Sentence>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.buffer.is_empty() {
            while let Some(sent) = self.inner.next() {
                match sent {
                    Ok(sent) => self.buffer.push(sent),
                    Err(err) => return Some(Err(err)),
                }

                if self.buffer.len() == self.buffer_size {
                    break;
                }
            }
        }

        match self.inner.next() {
            Some(sent) => match sent {
                Ok(sent) => Some(Ok(self.buffer.push_and_remove_random(sent))),
                Err(err) => Some(Err(err)),
            },
            None => self.buffer.remove_random().map(Result::Ok),
        }
    }
}
