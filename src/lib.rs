// Licensed to the Apache Software Foundation (ASF) under one
// or more contributor license agreements.  See the NOTICE file
// distributed with this work for additional information
// regarding copyright ownership.  The ASF licenses this file
// to you under the Apache License, Version 2.0 (the
// "License"); you may not use this file except in compliance
// with the License.  You may obtain a copy of the License at
//
//   http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing,
// software distributed under the License is distributed on an
// "AS IS" BASIS, WITHOUT WARRANTIES OR CONDITIONS OF ANY
// KIND, either express or implied.  See the License for the
// specific language governing permissions and limitations
// under the License.

use percent_encoding::percent_decode_str;
use pyo3::exceptions::{PyOSError, PyValueError};
use pyo3::prelude::*;
use pyo3::types::PyBytes;
use swhid::error::SwhidError;
use std::path::PathBuf;

fn swhid_err(e: SwhidError) -> PyErr {
    match e {
        SwhidError::Io(io_err) => PyOSError::new_err(io_err.to_string()),
        other => PyValueError::new_err(other.to_string()),
    }
}

// ---------------------------------------------------------------------------
// ObjectType enum
// ---------------------------------------------------------------------------

/// SWHID object type: cnt, dir, rev, rel, snp
#[pyclass(name = "ObjectType", eq, eq_int)]
#[derive(Clone, Debug, PartialEq)]
pub enum PyObjectType {
    Content = 0,
    Directory = 1,
    Revision = 2,
    Release = 3,
    Snapshot = 4,
}

#[pymethods]
impl PyObjectType {
    /// Return the three-letter tag (e.g. "cnt", "dir").
    fn tag(&self) -> &'static str {
        match self {
            PyObjectType::Content => "cnt",
            PyObjectType::Directory => "dir",
            PyObjectType::Revision => "rev",
            PyObjectType::Release => "rel",
            PyObjectType::Snapshot => "snp",
        }
    }

    fn __repr__(&self) -> String {
        format!("ObjectType.{}", match self {
            PyObjectType::Content => "Content",
            PyObjectType::Directory => "Directory",
            PyObjectType::Revision => "Revision",
            PyObjectType::Release => "Release",
            PyObjectType::Snapshot => "Snapshot",
        })
    }
}

impl From<swhid::ObjectType> for PyObjectType {
    fn from(ot: swhid::ObjectType) -> Self {
        match ot {
            swhid::ObjectType::Content => PyObjectType::Content,
            swhid::ObjectType::Directory => PyObjectType::Directory,
            swhid::ObjectType::Revision => PyObjectType::Revision,
            swhid::ObjectType::Release => PyObjectType::Release,
            swhid::ObjectType::Snapshot => PyObjectType::Snapshot,
        }
    }
}

impl From<PyObjectType> for swhid::ObjectType {
    fn from(ot: PyObjectType) -> Self {
        match ot {
            PyObjectType::Content => swhid::ObjectType::Content,
            PyObjectType::Directory => swhid::ObjectType::Directory,
            PyObjectType::Revision => swhid::ObjectType::Revision,
            PyObjectType::Release => swhid::ObjectType::Release,
            PyObjectType::Snapshot => swhid::ObjectType::Snapshot,
        }
    }
}

// ---------------------------------------------------------------------------
// Swhid – core identifier
// ---------------------------------------------------------------------------

/// A parsed SWHID core identifier (``swh:1:<type>:<hex>``).
#[pyclass(name = "Swhid")]
#[derive(Clone, Debug)]
pub struct PySwhid {
    inner: swhid::Swhid,
}

#[pymethods]
impl PySwhid {
    /// Parse a SWHID string such as ``"swh:1:cnt:abc123..."``.
    #[new]
    fn new(swhid_str: &str) -> PyResult<Self> {
        let inner: swhid::Swhid = swhid_str
            .parse()
            .map_err(swhid_err)?;
        Ok(PySwhid { inner })
    }

    /// The object type.
    #[getter]
    fn object_type(&self) -> PyObjectType {
        self.inner.object_type().into()
    }

    /// The 40-character lowercase hex digest.
    #[getter]
    fn digest_hex(&self) -> String {
        self.inner.digest_hex()
    }

    /// The raw 20-byte digest.
    fn digest_bytes<'py>(&self, py: Python<'py>) -> Bound<'py, PyBytes> {
        PyBytes::new(py, self.inner.digest_bytes())
    }

    fn __str__(&self) -> String {
        self.inner.to_string()
    }

    fn __repr__(&self) -> String {
        format!("Swhid('{}')", self.inner)
    }

    fn __eq__(&self, other: &PySwhid) -> bool {
        self.inner == other.inner
    }

    fn __hash__(&self) -> u64 {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};
        let mut h = DefaultHasher::new();
        self.inner.to_string().hash(&mut h);
        h.finish()
    }
}

// ---------------------------------------------------------------------------
// QualifiedSwhid
// ---------------------------------------------------------------------------

/// A SWHID with optional qualifiers (origin, visit, anchor, path, lines, bytes).
#[pyclass(name = "QualifiedSwhid")]
#[derive(Clone, Debug)]
pub struct PyQualifiedSwhid {
    inner: swhid::QualifiedSwhid,
}

impl PyQualifiedSwhid {
    fn qualifier_raw(&self, key: &str) -> Option<String> {
        let s = self.inner.to_string();
        let (_, rest) = s.split_once(';')?;
        for item in rest.split(';') {
            if let Some((k, v)) = item.split_once('=') {
                if k == key {
                    return Some(v.to_string());
                }
            }
        }
        None
    }

    fn qualifier_decoded(&self, key: &str) -> Option<String> {
        self.qualifier_raw(key).map(|v| {
            percent_decode_str(&v)
                .decode_utf8_lossy()
                .into_owned()
        })
    }

    fn qualifier_range(&self, key: &str) -> Option<(u64, Option<u64>)> {
        let v = self.qualifier_raw(key)?;
        match v.split_once('-') {
            Some((a, b)) => Some((a.parse().ok()?, Some(b.parse().ok()?))),
            None => Some((v.parse().ok()?, None)),
        }
    }
}

#[pymethods]
impl PyQualifiedSwhid {
    /// Parse a qualified SWHID string.
    #[new]
    fn new(s: &str) -> PyResult<Self> {
        let inner: swhid::QualifiedSwhid = s
            .parse()
            .map_err(swhid_err)?;
        Ok(PyQualifiedSwhid { inner })
    }

    /// The core SWHID (without qualifiers).
    #[getter]
    fn core(&self) -> PySwhid {
        PySwhid {
            inner: self.inner.core().clone(),
        }
    }

    /// The origin URL, or ``None``.
    #[getter]
    fn origin(&self) -> Option<String> {
        self.qualifier_decoded("origin")
    }

    /// The visit SWHID, or ``None``.
    #[getter]
    fn visit(&self) -> Option<PySwhid> {
        self.qualifier_raw("visit")
            .and_then(|v| v.parse::<swhid::Swhid>().ok())
            .map(|inner| PySwhid { inner })
    }

    /// The anchor SWHID, or ``None``.
    #[getter]
    fn anchor(&self) -> Option<PySwhid> {
        self.qualifier_raw("anchor")
            .and_then(|v| v.parse::<swhid::Swhid>().ok())
            .map(|inner| PySwhid { inner })
    }

    /// The path qualifier, or ``None``.
    #[getter]
    fn path(&self) -> Option<String> {
        self.qualifier_decoded("path")
    }

    /// The lines qualifier as ``(start, end)`` or ``(start, None)``, or ``None``.
    #[getter]
    fn lines(&self) -> Option<(u64, Option<u64>)> {
        self.qualifier_range("lines")
    }

    /// The bytes qualifier as ``(start, end)`` or ``(start, None)``, or ``None``.
    #[getter]
    fn bytes(&self) -> Option<(u64, Option<u64>)> {
        self.qualifier_range("bytes")
    }

    /// Return a new QualifiedSwhid with origin set.
    fn with_origin(&self, url: &str) -> Self {
        PyQualifiedSwhid { inner: self.inner.clone().with_origin(url) }
    }

    /// Return a new QualifiedSwhid with visit set.
    fn with_visit(&self, id: &PySwhid) -> Self {
        PyQualifiedSwhid { inner: self.inner.clone().with_visit(id.inner.clone()) }
    }

    /// Return a new QualifiedSwhid with anchor set.
    fn with_anchor(&self, id: &PySwhid) -> Self {
        PyQualifiedSwhid { inner: self.inner.clone().with_anchor(id.inner.clone()) }
    }

    /// Return a new QualifiedSwhid with path set.
    fn with_path(&self, path: &str) -> PyResult<Self> {
        let q = self.inner.clone().with_path(path);
        Ok(PyQualifiedSwhid { inner: q })
    }

    /// Return a new QualifiedSwhid with lines set.
    #[pyo3(signature = (start, end=None))]
    fn with_lines(&self, start: u64, end: Option<u64>) -> PyResult<Self> {
        let range = swhid::LineRange { start, end };
        let q = self.inner.clone().with_lines(range);
        Ok(PyQualifiedSwhid { inner: q })
    }

    /// Return a new QualifiedSwhid with bytes set.
    #[pyo3(signature = (start, end=None))]
    fn with_bytes(&self, start: u64, end: Option<u64>) -> PyResult<Self> {
        let range = swhid::ByteRange { start, end };
        let q = self.inner.clone().with_bytes(range);
        Ok(PyQualifiedSwhid { inner: q })
    }

    fn __str__(&self) -> String {
        self.inner.to_string()
    }

    fn __eq__(&self, other: &PyQualifiedSwhid) -> bool {
        self.inner == other.inner
    }

    fn __hash__(&self) -> u64 {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};
        let mut h = DefaultHasher::new();
        self.inner.to_string().hash(&mut h);
        h.finish()
    }

    fn __repr__(&self) -> String {
        format!("QualifiedSwhid('{}')", self.inner)
    }
}

// ---------------------------------------------------------------------------
// Free functions – content & directory hashing
// ---------------------------------------------------------------------------

/// Compute the SWHID for raw byte content (file data).
///
/// This hashes data as a Git blob, producing a ``swh:1:cnt:…`` identifier.
///
/// Args:
///     data: The raw bytes of the file.
///
/// Returns:
///     A ``Swhid`` object.
#[pyfunction]
fn content_id(data: &[u8]) -> PySwhid {
    let inner = swhid::Content::from_bytes(data).swhid();
    PySwhid { inner }
}

/// Compute the SWHID for a file on disk.
///
/// Args:
///     path: Filesystem path to the file.
///
/// Returns:
///     A ``Swhid`` object.
#[pyfunction]
fn content_id_from_file(path: &str) -> PyResult<PySwhid> {
    let data = std::fs::read(path)
        .map_err(|e| PyOSError::new_err(format!("cannot read {path}: {e}")))?;
    Ok(content_id(&data))
}

/// Compute the SWHID for a directory tree.
///
/// This computes the Merkle hash over the directory following Git's tree
/// object format, producing a ``swh:1:dir:…`` identifier.
///
/// Args:
///     root: Path to the directory.
///     follow_symlinks: Whether to follow symlinks (default False).
///     exclude_suffixes: File suffixes to skip (e.g. ``[".pyc", ".o"]``).
///
/// Returns:
///     A ``Swhid`` object.
#[pyfunction]
#[pyo3(signature = (root, follow_symlinks=false, exclude_suffixes=None))]
fn directory_id(
    root: &str,
    follow_symlinks: bool,
    exclude_suffixes: Option<Vec<String>>,
) -> PyResult<PySwhid> {
    let path = PathBuf::from(root);
    let walk_opts = swhid::WalkOptions {
        follow_symlinks,
        exclude_suffixes: exclude_suffixes.unwrap_or_default(),
    };
    let inner = swhid::DiskDirectoryBuilder::new(&path)
        .with_options(walk_opts)
        .swhid()
        .map_err(swhid_err)?;
    Ok(PySwhid { inner })
}

/// Verify that a file or directory matches an expected SWHID.
///
/// Args:
///     path: Filesystem path to a file or directory.
///     expected: The SWHID string to compare against.
///     follow_symlinks: Whether to follow symlinks (default False, directories only).
///     exclude_suffixes: File suffixes to skip (directories only).
///
/// Returns:
///     ``True`` if the computed SWHID matches, ``False`` otherwise.
#[pyfunction]
#[pyo3(signature = (path, expected, follow_symlinks=false, exclude_suffixes=None))]
fn verify(
    path: &str,
    expected: &str,
    follow_symlinks: bool,
    exclude_suffixes: Option<Vec<String>>,
) -> PyResult<bool> {
    let expected_swhid: swhid::Swhid = expected
        .parse()
        .map_err(swhid_err)?;

    let p = PathBuf::from(path);
    let computed = if p.is_dir() {
        let walk_opts = swhid::WalkOptions {
            follow_symlinks,
            exclude_suffixes: exclude_suffixes.unwrap_or_default(),
        };
        swhid::DiskDirectoryBuilder::new(&p)
            .with_options(walk_opts)
            .swhid()
            .map_err(swhid_err)?
    } else {
        let data = std::fs::read(&p)
            .map_err(|e| PyOSError::new_err(format!("cannot read {path}: {e}")))?;
        swhid::Content::from_bytes(data).swhid()
    };

    Ok(computed == expected_swhid)
}

// ---------------------------------------------------------------------------
// Module
// ---------------------------------------------------------------------------

/// Python bindings for the ``swhid-rs`` SWHID v1.2 reference implementation.
///
/// Provides content-addressed identifiers (SWHIDs) compatible with the
/// Software Heritage archive and ISO/IEC 18670:2025.
#[pymodule]
fn swhid_py(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_class::<PyObjectType>()?;
    m.add_class::<PySwhid>()?;
    m.add_class::<PyQualifiedSwhid>()?;
    m.add_function(wrap_pyfunction!(content_id, m)?)?;
    m.add_function(wrap_pyfunction!(content_id_from_file, m)?)?;
    m.add_function(wrap_pyfunction!(directory_id, m)?)?;
    m.add_function(wrap_pyfunction!(verify, m)?)?;
    Ok(())
}