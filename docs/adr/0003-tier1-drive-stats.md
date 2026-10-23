# ADR-3: Privilege-free drive stats live in the server

**Status:** Accepted (v1.15.6)

## Context

The Drives dashboard already had capacity per filesystem, computed in the TypeScript server with
`statfsSync`. Tier-1 hardware counters (model, serial, SSD/HDD, bus, temperature, I/O counters,
inodes) are readable from sysfs and procfs without opening a block device, but they need a path to
a filesystem and its `st_dev`, which the server already has. SMART needs raw device access and is
out of scope for this tier.

## Decision

Collect Tier-1 stats in `apps/server/src/disk-stats.ts` and attach them to `DriveInfo` as an
optional `stats` object:

- `/proc/diskstats` and `/sys/class/block` are host-wide, so a container sees them without a
  `/dev` mount or extra capabilities.
- `st_dev` of a mount resolves through `/sys/dev/block/<major>:<minor>` to the kernel device;
  device-mapper and RAID volumes follow `slaves/` to the physical disks for model and sensors.
- Temperature comes from the hwmon entry bound to the same device node; no sensor means the field
  is absent, not zero.
- Filesystem type and source come from the container's own `/proc/self/mounts`, which names the
  filesystem behind a bind mount even though it cannot enumerate host mounts.
- The Rust core keeps owning scan/index/query; no IPC or index-format change is needed.

## Consequences

- The default compose keeps its read-only bind mount and adds no privileges.
- Every field is optional and the collector returns `undefined` on non-Linux hosts or virtual
  devices, so the UI degrades to capacity-only instead of failing.
- SMART (and self-tests) would require `/dev`, `CAP_SYS_RAWIO` and possibly `CAP_SYS_ADMIN`; that
  belongs in a separate, explicitly enabled feature, not here.
