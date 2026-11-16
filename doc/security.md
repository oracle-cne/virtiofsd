# Security considerations

Several aspects of virtiofsd can expose host files or weaken guest
security if not mitigated:

1. **[Limitations of chroot mode](#limitations-of-chroot-mode)**.
   In chroot mode, virtiofsd keeps open descriptors that point outside
   the shared directory.  These survive the `chroot` call.
   A **compromised** process can use them to access the host root
   filesystem.

2. **[File handle security](#file-handle-security)**.  When file handles
   are enabled (the default), virtiofsd can open any file on the shared
   directory's filesystem.  If that filesystem is the host root
   filesystem, this may include all host files.

3. **[`KILLPRIV_V2` limitations](#killpriv_v2-limitations)**.
   `KILLPRIV_V2` is a performance feature that delegates `SUID`/`SGID`
   stripping to virtiofsd.  The kernel implementation is incomplete and
   can lead to privilege escalation in the guest.

This document explains these issues and how to mitigate them.

## Summary

- Use `--sandbox=namespace` (the default).  Use `--sandbox=chroot` only
  inside a container.
  See [Limitations of chroot mode](#limitations-of-chroot-mode).
- Put the shared directory on a separate filesystem.
  See [Why sandboxing alone does not contain file handles](#why-sandboxing-alone-does-not-contain-file-handles).
- Place the socket and pidfile on the same filesystem as the shared
  directory or on a tmpfs.
- Protect the migration channel.
- Restore only trusted snapshots.
- Disable file handles if not needed (at the cost of higher descriptor
  usage).
- Do not enable `--killpriv-v2` on VMs with untrusted users.
  See [`KILLPRIV_V2` limitations](#killpriv_v2-limitations).

## Limitations of chroot mode

virtiofsd provides two sandbox modes:

- `--sandbox=namespace` (default): creates new mount, PID, and network
  namespaces (and a user namespace if unprivileged), then calls
  `pivot_root`.
- `--sandbox=chroot`: calls `chroot(2)` into the shared directory.

Before calling `chroot(2)`, virtiofsd opens descriptors to the
host's procfs for normal operation.  It may also hold descriptors to
its socket and pidfile.  The `chroot` call changes the root for path
resolution but does not close existing descriptors.  A compromised
process can follow these descriptors to the host root filesystem.

Use `--sandbox=chroot` only inside a container that already provides
namespace isolation.  Prefer `--sandbox=namespace` in all other cases.

## File handle security

By default (`--inode-file-handles=prefer`), virtiofsd stores file
handles instead of `O_PATH` file descriptors.  A file handle is a
token that identifies a file.  `name_to_handle_at(2)` returns
a handle and `open_by_handle_at(2)` opens a file using that handle.

Instead of walking the directory tree, the kernel finds the file by
looking up its inode number directly in the filesystem.
A process with the right handle can open any file on that filesystem.

Because `open_by_handle_at(2)` bypasses directory permissions, the
kernel requires `CAP_DAC_READ_SEARCH` in the initial user namespace.
virtiofsd retains this capability by default when file handles are
enabled.  Pass `--inode-file-handles=never` to drop it and use `O_PATH`
file descriptors only.  In a user namespace without privileges,
virtiofsd lacks this capability and disables file handles automatically.

### Why sandboxing alone does not contain file handles

No sandbox mode limits file handle scope.  Bind mounts, `chroot`, and
`pivot_root` create views of a directory tree but share the original
superblock.  A process can still open any file on that filesystem
through `open_by_handle_at(2)`.

The security boundary for file handles is the **filesystem**
(superblock), not the directory.  You must assume that virtiofsd can
access every file on that filesystem, not just the shared directory
subtree.  Additionally, virtiofsd opens its socket and pidfile before
entering the sandbox.  These descriptors survive sandboxing.
A compromised process can combine these descriptors with a file handle
to access any file on their filesystem.

A separate filesystem combined with namespace sandboxing limits what
file handles can reach.  The shared directory must sit on a different
filesystem than the host root.  It can be the filesystem root or a
subdirectory.  Nested mounts inside the shared directory also
expose their filesystems to file handle resolution.  Place the socket
and pidfile on the same filesystem as the shared directory or on a
tmpfs.

With `--migration-mode=file-handles`, migration streams and snapshots
also carry file handles.  An attacker who controls the stream can
inject forged handles.

## `KILLPRIV_V2` limitations

`--killpriv-v2` skips a `GETXATTR` round trip on every write.  It is
off by default because the kernel implementation is incomplete.  It
can lead to privilege escalation in the guest.

When a user without `CAP_FSETID` writes to a `SUID` file, the kernel
strips the `SUID` bit.  It also strips the `SGID` bit if the file is
group-executable.  With `KILLPRIV_V2`, virtiofsd strips the bits
instead of the kernel.  If virtiofsd fails to strip the bits, they
stay.

Problems:

- **Remote host filesystems**.  virtiofsd drops `CAP_FSETID` before
  each host syscall so the host kernel strips the bits.  On NFS,
  CIFS, or other network filesystems the remote server ignores the
  dropped capability.  The bits stay.
- **Missing flags**.  The FUSE protocol has no flag for `fallocate(2)`
  or `copy_file_range(2)`.  virtiofsd always drops `CAP_FSETID` for
  these calls, so it strips the bits even when the file owner should
  keep them.
- **Other corner cases**.  The rules for stripping `SUID`/`SGID` bits
  are complex and depend on file ownership, group membership,
  capabilities, and POSIX ACLs.  Other unidentified gaps may exist.

Do not use `--killpriv-v2` on VMs where untrusted users can write to
the shared directory.
