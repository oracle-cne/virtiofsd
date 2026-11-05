use crate::Error;
use crate::{fuse, util};

use std::convert::TryFrom;
use std::ffi::CStr;
use std::ops::Deref;

/// A validated single-component directory entry name.
/// Rejects ".", "..", "/", and empty names.
#[derive(Clone, Copy, Debug)]
pub struct FileName<'a>(&'a CStr);

// Deref is safe here because FileName validates at construction time.
// The inner CStr is always a valid directory entry name. FileName
// does not shadow any CStr method.
impl Deref for FileName<'_> {
    type Target = CStr;

    fn deref(&self) -> &CStr {
        self.0
    }
}

impl<'a> TryFrom<&'a [u8]> for FileName<'a> {
    type Error = crate::Error;

    fn try_from(buf: &'a [u8]) -> crate::Result<Self> {
        let name = util::bytes_to_cstr(buf)?;
        let bytes = name.to_bytes();

        if bytes.is_empty() || bytes.contains(&b'/') || bytes == b"." || bytes == b".." {
            return Err(Error::InvalidFileName);
        }

        Ok(Self(name))
    }
}

/// A validated name for FUSE lookup operations.
/// Allows "." and ".." for NFS export support.
/// Rejects "/", and empty names.
/// When the parent is the root inode (1), ".." resolves to ".".
#[derive(Clone, Copy, Debug)]
pub struct LookupName<'a>(&'a CStr);

// SAFETY: b".\0" is a valid NUL-terminated string with no interior NUL.
const DOT: &CStr = unsafe { CStr::from_bytes_with_nul_unchecked(b".\0") };

impl<'a> LookupName<'a> {
    pub fn try_new(parent: u64, buf: &'a [u8]) -> crate::Result<Self> {
        let name = util::bytes_to_cstr(buf)?;
        let bytes = name.to_bytes();

        if bytes.is_empty() || bytes.contains(&b'/') {
            return Err(Error::InvalidLookupName);
        }

        // NFS export sends "." lookups to resolve file handles (see kernel's fuse_get_dentry())
        // and ".." lookups to find parent directories (see kernel's fuse_get_parent()).
        // Do not allow escaping the shared directory.
        if parent == fuse::ROOT_ID && bytes == b".." {
            Ok(Self(DOT))
        } else {
            Ok(Self(name))
        }
    }

    #[inline]
    pub fn is_dot(&self) -> bool {
        self.0.to_bytes() == b"."
    }

    #[inline]
    pub fn is_dotdot(&self) -> bool {
        self.0.to_bytes() == b".."
    }
}

// Deref is safe here because LookupName validates at construction time.
// The inner CStr is always a valid directory entry name.
// LookupName has no public methods that shadow CStr methods.
impl Deref for LookupName<'_> {
    type Target = CStr;

    fn deref(&self) -> &CStr {
        self.0
    }
}

// FileName rejects everything LookupName rejects, plus "." and "..".
// The conversion is always valid.
impl<'a> From<FileName<'a>> for LookupName<'a> {
    fn from(name: FileName<'a>) -> Self {
        Self(name.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::ffi::CString;

    fn cstr_bytes(s: &str) -> Vec<u8> {
        // We cannot use c"" literals with edition 2018
        CString::new(s).unwrap().into_bytes_with_nul()
    }

    #[test]
    fn filename_accepts_valid() {
        let cases = &["file", ".hidden", "..foo"];
        for name in cases {
            let buf = cstr_bytes(name);
            assert!(
                FileName::try_from(buf.as_slice()).is_ok(),
                "should accept {:?}",
                name
            );
        }
    }

    #[test]
    fn filename_rejects_invalid() {
        let cases = &[
            "",
            ".",
            "..",
            "/",
            "a/b",
            "/etc/passwd",
            "../../../tmp/secret",
        ];
        for name in cases {
            let buf = cstr_bytes(name);
            assert!(
                FileName::try_from(buf.as_slice()).is_err(),
                "should reject {:?}",
                name
            );
        }
    }

    #[test]
    fn filename_rejects_missing_nul_terminator() {
        assert!(FileName::try_from(b"hello" as &[u8]).is_err());
    }

    #[test]
    fn filename_rejects_interior_nul() {
        assert!(FileName::try_from(b"hel\0lo\0" as &[u8]).is_err());
    }

    #[test]
    fn lookup_name_accepts_valid() {
        let cases = &["file", ".", "..", ".hidden", "..foo"];
        for name in cases {
            let buf = cstr_bytes(name);
            assert!(
                LookupName::try_new(2, &buf).is_ok(),
                "should accept {:?}",
                name
            );
        }
    }

    #[test]
    fn lookup_name_resolves_dotdot_at_root() {
        let buf = cstr_bytes("..");
        let name = LookupName::try_new(fuse::ROOT_ID, &buf).unwrap();
        assert_eq!(name.to_bytes(), b".");
    }

    #[test]
    fn lookup_name_preserves_dotdot_for_non_root() {
        let buf = cstr_bytes("..");
        let name = LookupName::try_new(2, &buf).unwrap();
        assert_eq!(name.to_bytes(), b"..");
    }

    #[test]
    fn lookup_name_preserves_dot_at_root() {
        let buf = cstr_bytes(".");
        let name = LookupName::try_new(fuse::ROOT_ID, &buf).unwrap();
        assert_eq!(name.to_bytes(), b".");
    }

    #[test]
    fn lookup_name_preserves_regular_name_at_root() {
        let buf = cstr_bytes("file");
        let name = LookupName::try_new(fuse::ROOT_ID, &buf).unwrap();
        assert_eq!(name.to_bytes(), b"file");
    }

    #[test]
    fn lookup_name_rejects_invalid() {
        let cases = &["", "/", "a/b", "/etc/passwd", "../../../tmp/secret"];
        for name in cases {
            let buf = cstr_bytes(name);
            assert!(
                LookupName::try_new(2, &buf).is_err(),
                "should reject {:?}",
                name
            );
        }
    }

    #[test]
    fn lookup_name_rejects_missing_nul_terminator() {
        assert!(LookupName::try_new(2, b"hello").is_err());
    }

    #[test]
    fn lookup_name_rejects_interior_nul() {
        assert!(LookupName::try_new(2, b"hel\0lo\0").is_err());
    }

    #[test]
    fn filename_converts_to_lookup_name() {
        let buf = cstr_bytes("file");
        let fname = FileName::try_from(buf.as_slice()).unwrap();
        let lname: LookupName = fname.into();
        assert_eq!(lname.to_bytes(), b"file");
    }
}
