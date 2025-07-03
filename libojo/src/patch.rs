// Copyright 2018-2019 Joe Neeman.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.
//
// See the LICENSE-APACHE or LICENSE-MIT files at the top-level directory
// of this distribution.

use {
    chrono::{DateTime, Utc},
    sha2::{Digest, Sha256},
    std::{
        collections::HashSet,
        io::{self, prelude::*},
    },
};

use crate::{Error, error::PatchIdError};

mod change;
pub use self::change::{Change, Changes};

// This is just a wrapper around some instance of io::Write that calculates a hash of everything
// that's written.
struct HashingWriter<W: Write> {
    writer: W,
    hasher: Sha256,
}

impl<W: Write> HashingWriter<W> {
    fn new(writer: W) -> HashingWriter<W> {
        HashingWriter {
            writer,
            hasher: Default::default(),
        }
    }
}

impl<W: Write> Write for HashingWriter<W> {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        self.hasher.input(buf);
        self.writer.write(buf)
    }

    fn flush(&mut self) -> io::Result<()> {
        self.writer.flush()
    }
}

struct HashingReader<R: Read> {
    reader: R,
    hasher: Sha256,
}

impl<R: Read> HashingReader<R> {
    fn new(reader: R) -> HashingReader<R> {
        HashingReader {
            reader,
            hasher: Default::default(),
        }
    }
}

impl<R: Read> Read for HashingReader<R> {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        let size = self.reader.read(buf)?;
        self.hasher.input(&buf[..size]);
        Ok(size)
    }
}

// PatchId contains a [u8; 32], which by default serializes to an array in yaml (and other
// human-readable formats). To make the output more compact and readable, it's better to convert it
// to a base64 string.
mod patch_id_base64 {
    pub fn serialize<S>(bytes: &[u8], serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        if serializer.is_human_readable() {
            serializer.serialize_str(&base64::encode_config(bytes, base64::URL_SAFE))
        } else {
            serializer.serialize_bytes(bytes)
        }
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<[u8; 32], D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        if deserializer.is_human_readable() {
            let s = <String as serde::Deserialize>::deserialize(deserializer)?;
            let mut ret = [0; 32];
            let vec =
                base64::decode_config(&s, base64::URL_SAFE).map_err(serde::de::Error::custom)?;
            ret.copy_from_slice(&vec[..]);
            Ok(ret)
        } else {
            <[u8; 32] as serde::Deserialize>::deserialize(deserializer)
        }
    }
}

/// A global identifier for a patch.
///
/// A `PatchId` is derived from a patch by hashing its contents. It must be unique: a repository
/// cannot simultaneously contain two patches with the same id.
#[derive(Copy, Clone, Deserialize, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(transparent)]
pub struct PatchId {
    #[serde(with = "patch_id_base64")]
    pub(crate) data: [u8; 32],
}

impl std::fmt::Debug for PatchId {
    fn fmt(&self, fmt: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        fmt.debug_tuple("PatchId").field(&self.to_base64()).finish()
    }
}

impl PatchId {
    /// There is a special reserved `PatchId` for patches that are under construction, but not yet
    /// finished (see [`UnidentifiedPatch`] for more details). This function returns that special id.
    pub fn cur() -> PatchId {
        PatchId { data: [0; 32] }
    }

    /// Checks whether this `PatchId` is the one decribed in [`PatchId::cur`].
    pub fn is_cur(&self) -> bool {
        self.data == [0; 32]
    }

    /// Represents this `PatchId` in base64.
    ///
    /// We encode in the URL_SAFE encoding because it needs to be a valid path (e.g. no
    /// slashes). We prepend the letter 'P', because otherwise there's a chance that
    /// the first character will be '-', which is annoying because then the CLI might
    /// misinterpret it as a flag.
    pub fn to_base64(&self) -> String {
        // base64 requires 44 characters to represent 32 bytes. Add one for the 'P'.
        let mut ret = vec![0; 45];
        ret[0] = b'P';
        base64::encode_config_slice(&self.data[..], base64::URL_SAFE, &mut ret[1..]);

        // We can safely unwrap because base64 is guaranteed to be ASCII.
        String::from_utf8(ret).unwrap()
    }

    /// Converts from base64 (as returned by [`PatchId::to_base64`]) to a `PatchId`.
    pub fn from_base64<S: ?Sized + AsRef<[u8]>>(name: &S) -> Result<PatchId, Error> {
        let data = base64::decode_config(&name.as_ref()[1..], base64::URL_SAFE)
            .map_err(PatchIdError::from)?;
        let mut ret = PatchId::cur();
        if data.len() != ret.data.len() {
            Err(PatchIdError::InvalidLength(data.len()).into())
        } else {
            ret.data.copy_from_slice(&data);
            Ok(ret)
        }
    }

    // Creates a PatchId from a Sha256 hasher
    fn from_sha256(hasher: Sha256) -> PatchId {
        let mut ret = PatchId::cur();
        ret.data.copy_from_slice(&hasher.result()[..]);
        ret
    }
}

/// Like a [`Patch`], but without the unique id.
///
/// A patch is ultimately identified by its id, which is generated by hashing the contents of the
/// serialized patch. This ends up being a bit circular, because the contents of the patch might
/// actually depend on the id, and those contents in turn will affect the id. The way we break this
/// cycle is by separating "unidentified" patches (those without an id yet) from completed patches
/// with an id.
///
/// This is an unidentified patch; it does not have an id field, and any changes that need
/// to refer to contents of this patch use the placeholder id returned by [`PatchId::cur`].
///
/// This patch *cannot* be applied to a repository, because doing so would require an id. However,
/// it can be serialized to a file, and it can be turned into an identified patch.
#[derive(Clone, Debug, Deserialize, Eq, Hash, PartialEq, Serialize)]
pub struct UnidentifiedPatch {
    changes: Changes,

    // Various metadata associated with this patch.
    //
    // Note that the metadata is hashed together will all the other contents of this patch. So,
    // for example, if you change the author of a patch then the resulting patch id will also
    // change.
    header: PatchHeader,

    // The list of other patches on which this depends. This should coincide with the set of all
    // other PatchIds that are referenced in `changes`.
    deps: Vec<PatchId>,
}

impl UnidentifiedPatch {
    /// Creates a new `UnidentifiedPatch` from some metadata and a set of changes.
    pub fn new(author: String, description: String, changes: Changes) -> UnidentifiedPatch {
        // The dependencies of this patch consist of all patches that are referred to by the list
        // of changes.
        let mut deps = HashSet::new();
        for c in &changes.changes {
            match *c {
                Change::DeleteNode { ref id } => {
                    if !id.patch.is_cur() {
                        deps.insert(id.patch);
                    }
                }
                Change::NewEdge { ref src, ref dest } => {
                    if !src.patch.is_cur() {
                        deps.insert(src.patch);
                    }
                    if !dest.patch.is_cur() {
                        deps.insert(dest.patch);
                    }
                }
                _ => {}
            }
        }

        UnidentifiedPatch {
            header: PatchHeader {
                author,
                description,
                #[cfg(not(target_arch = "wasm32"))]
                timestamp: Utc::now(),
            },
            changes,
            deps: deps.into_iter().collect(),
        }
    }

    // Assigns an id to this UnidentifiedPatch, and in doing so turns it into a Patch.
    fn set_id(self, id: PatchId) -> Patch {
        let mut ret = Patch {
            id,
            header: self.header,
            changes: self.changes,
            deps: self.deps,
        };

        ret.changes.set_patch_id(&ret.id);
        ret
    }

    /// Writes out a patch.
    ///
    /// While writing out the patch, we compute the hash of its contents and use that to derive an
    /// id for this patch. Assuming that the writing succeeds, we return the resulting [`Patch`].
    pub fn write_out<W: Write>(self, writer: W) -> Result<Patch, serde_yaml::Error> {
        let mut w = HashingWriter::new(writer);
        serde_yaml::to_writer(&mut w, &self)?;

        let patch_id = PatchId::from_sha256(w.hasher);
        Ok(self.set_id(patch_id))
    }
}

/// A set of changes together with some metadata (author, description, etc.) and a unique id.
///
/// There are two ways to create a patch:
/// - use [`Patch::from_reader`] to read a patch from some input, or
/// - use [`UnidentifiedPatch::write_out`] to turn an [`UnidentifiedPatch`] into a real patch, by
///   computing its id.
#[derive(Clone, Debug, Deserialize, Eq, Hash, PartialEq)]
pub struct Patch {
    id: PatchId,
    header: PatchHeader,
    changes: Changes,
    deps: Vec<PatchId>,
}

impl Patch {
    /// Creates a patch by deserializing it from a reader.
    ///
    /// The id of the resulting patch will be the SHA256 hash of the contents.
    pub fn from_reader<R: Read>(input: R) -> Result<Patch, Error> {
        let mut reader = HashingReader::new(input);
        let up: UnidentifiedPatch = serde_yaml::from_reader(&mut reader)?;
        let id = PatchId::from_sha256(reader.hasher);
        Ok(up.set_id(id))
    }

    /// The unique id of this patch.
    pub fn id(&self) -> &PatchId {
        &self.id
    }

    /// The patch header.
    pub fn header(&self) -> &PatchHeader {
        &self.header
    }

    /// The changes that this patch makes.
    pub fn changes(&self) -> &Changes {
        &self.changes
    }

    /// The dependencies of this patch.
    ///
    /// Before this patch can be applied, all of its dependencies must already have been applied.
    pub fn deps(&self) -> &[PatchId] {
        &self.deps
    }
}

/// Various metadata associated with a patch.
///
/// This data does not affect the changes that a patch actually makes, but it is considered part of
/// the patch as far as hashing is concerned. In particular, if you change any of this metadata
/// then the result is a completely different patch.
#[derive(Clone, Debug, Deserialize, Eq, Hash, PartialEq, Serialize)]
pub struct PatchHeader {
    /// Author of the patch.
    pub author: String,

    /// A description of the patch.
    pub description: String,

    /// The time at which the patch was created.
    // We currently disable this on wasm, since chrono::Utc::now() panics there.
    #[cfg(not(target_arch = "wasm32"))]
    pub timestamp: DateTime<Utc>,
}
