# Licensed to the Apache Software Foundation (ASF) under one
# or more contributor license agreements.  See the NOTICE file
# distributed with this work for additional information
# regarding copyright ownership.  The ASF licenses this file
# to you under the Apache License, Version 2.0 (the
# "License"); you may not use this file except in compliance
# with the License.  You may obtain a copy of the License at
#
#   http://www.apache.org/licenses/LICENSE-2.0
#
# Unless required by applicable law or agreed to in writing,
# software distributed under the License is distributed on an
# "AS IS" BASIS, WITHOUT WARRANTIES OR CONDITIONS OF ANY
# KIND, either express or implied.  See the License for the
# specific language governing permissions and limitations
# under the License.

"""
Python bindings for the ``swhid-rs`` SWHID v1.2 reference implementation.

Provides content-addressed identifiers (SWHIDs) compatible with the
Software Heritage archive and ISO/IEC 18670:2025.

Wraps the Rust reference implementation at https://github.com/swhid/swhid-rs
using PyO3, giving you native-speed hashing with a Pythonic API.
"""

from __future__ import annotations

import enum
from typing import Optional

class ObjectType(enum.IntEnum):
    """SWHID object type tag."""

    Content = 0
    """File content (``cnt``)."""
    Directory = 1
    """Directory tree (``dir``)."""
    Revision = 2
    """VCS commit / changeset (``rev``)."""
    Release = 3
    """VCS annotated tag / release (``rel``)."""
    Snapshot = 4
    """Repository snapshot (``snp``)."""

    def tag(self) -> str:
        """Return the three-letter SWHID tag (e.g. ``"cnt"``)."""
        ...

class Swhid:
    """
    A parsed SWHID core identifier (``swh:1:<type>:<hex>``).

    Construct by parsing a string::

        s = Swhid("swh:1:cnt:b45ef6fec89518d314f546fd6c3025367b721684")

    Or obtain from :func:`content_id` / :func:`directory_id`.
    """

    def __init__(self, swhid_str: str) -> None:
        """Parse a SWHID string.

        Raises:
            ValueError: If the string is not a valid SWHID.
        """
        ...

    @property
    def object_type(self) -> ObjectType:
        """The object type (content, directory, etc.)."""
        ...

    @property
    def digest_hex(self) -> str:
        """The 40-character lowercase hex digest."""
        ...

    def digest_bytes(self) -> bytes:
        """The raw 20-byte SHA-1 digest."""
        ...

    def __str__(self) -> str: ...
    def __repr__(self) -> str: ...
    def __eq__(self, other: object) -> bool: ...
    def __hash__(self) -> int: ...

class QualifiedSwhid:
    """
    A SWHID with optional qualifiers (origin, visit, anchor, path, lines, bytes).

    Parse from a string::

        q = QualifiedSwhid("swh:1:cnt:...;origin=https://github.com/user/repo")

    Or build from a core Swhid via :meth:`with_origin`, :meth:`with_path`, etc.
    """

    def __init__(self, s: str) -> None:
        """Parse a qualified SWHID string.

        Raises:
            ValueError: If the string is not a valid qualified SWHID.
        """
        ...

    @property
    def core(self) -> Swhid:
        """The core SWHID (without qualifiers)."""
        ...

    def with_origin(self, url: str) -> QualifiedSwhid:
        """Return a copy with the ``origin`` qualifier set."""
        ...

    def with_path(self, path: str) -> QualifiedSwhid:
        """Return a copy with the ``path`` qualifier set."""
        ...

    def with_lines(self, start: int, end: Optional[int] = None) -> QualifiedSwhid:
        """Return a copy with the ``lines`` qualifier set."""
        ...

    def with_bytes(self, start: int, end: Optional[int] = None) -> QualifiedSwhid:
        """Return a copy with the ``bytes`` qualifier set."""
        ...

    def __str__(self) -> str: ...
    def __repr__(self) -> str: ...

def content_id(data: bytes) -> Swhid:
    """
    Compute the SWHID for raw byte content (file data).

    Hashes data as a Git blob, producing a ``swh:1:cnt:…`` identifier.

    Example::

        swhid = content_id(b"Hello, World!")
        print(swhid)  # swh:1:cnt:b45ef6fec89518d314f546fd6c3025367b721684
    """
    ...

def content_id_from_file(path: str) -> Swhid:
    """
    Compute the SWHID for a file on disk.

    Args:
        path: Filesystem path to the file.

    Raises:
        ValueError: If the file cannot be read.
    """
    ...

def directory_id(
    root: str,
    follow_symlinks: bool = False,
    exclude_suffixes: Optional[list[str]] = None,
) -> Swhid:
    """
    Compute the SWHID for a directory tree.

    Computes the Merkle hash over the directory following Git's tree object
    format, producing a ``swh:1:dir:…`` identifier.  Because the algorithm
    is format-agnostic, you can compare a ``.tar.gz`` and a ``.zip`` of the
    same release and confirm they have identical content.

    Args:
        root: Path to the directory.
        follow_symlinks: Whether to follow symlinks (default ``False``).
        exclude_suffixes: File suffixes to skip (e.g. ``[".pyc", ".o"]``).

    Raises:
        ValueError: If the directory cannot be traversed.
    """
    ...

def verify(path: str, expected: str) -> bool:
    """
    Verify that a file or directory matches an expected SWHID.

    Automatically detects whether *path* is a file or directory and
    computes the appropriate SWHID to compare.

    Args:
        path: Filesystem path to a file or directory.
        expected: The SWHID string to compare against.

    Returns:
        ``True`` if the computed SWHID matches, ``False`` otherwise.

    Raises:
        ValueError: If *expected* is not a valid SWHID or *path* cannot be read.
    """
    ...
