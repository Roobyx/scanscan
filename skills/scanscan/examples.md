# scanscan examples

All examples are read-only and JSON-first.

## 1. "The disk is full — what is using it?"

```bash
scanscan scan / --json > /tmp/scan.json
scanscan du / --depth 2 --json
scanscan top / -n 25 --metric alloc --json
```

Report the top 25 by allocated size with paths, plus any scan errors (unreadable dirs).

## 2. "What is under /var taking space?"

```bash
scanscan du /var --depth 3 --json
scanscan ext /var --json
```

## 3. Large, stale files (cleanup candidates)

```bash
scanscan find / --size +1G --mtime -180d --json
```

Report candidates only — never delete without explicit human confirmation.

## 4. Docker disk audit

```bash
scanscan docker --json
scanscan docker --stats --json
```

Lists container/image/volume sizes and mount → host-path mapping. Use the mount map to see which
container owns a given region of the disk.

## 5. "What grew this week?"

```bash
scanscan snapshots list --json
scanscan diff <older-snapshot> <newer-snapshot> --json
```

## 6. Agent HTTP flow

```bash
curl -s http://127.0.0.1:8080/api/v1/scans -X POST \
  -H 'content-type: application/json' \
  -d '{"roots":["/var"],"options":{"incremental":true}}'
curl -s http://127.0.0.1:8080/api/v1/scans/<id>/top?n=20
```
