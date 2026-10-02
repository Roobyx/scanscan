---
title: HTTP API
description: The REST/SSE API exposed by the server, base path /api/v1.
---

All endpoints are read-only except scan creation, cancellation and snapshot GC.

## Core

```
GET  /api/v1/health
GET  /api/v1/version
GET  /api/v1/config
```

## Scans

```
GET    /api/v1/scans
POST   /api/v1/scans                 # { roots: string[], options?: {...} }
GET    /api/v1/scans/:id
DELETE /api/v1/scans/:id
POST   /api/v1/scans/:id/cancel
GET    /api/v1/scans/:id/progress    # JSON, or SSE with Accept: text/event-stream
```

## Tree and queries

```
GET /api/v1/scans/:id/children/:nodeId?sort=size&limit=&offset=
GET /api/v1/scans/:id/tiles?scope=&depth=&color=ext|size
GET /api/v1/scans/:id/tree?scope=&depth=&limit=
GET /api/v1/scans/:id/top?n=&kind=&metric=
GET /api/v1/scans/:id/extensions
GET /api/v1/scans/:id/histogram?dim=ext|age|owner|size&scope=
GET /api/v1/scans/:id/owners
GET /api/v1/scans/:id/age
GET /api/v1/scans/:id/search?q=&ext=&kind=&size_min=&size_max=&limit=
GET /api/v1/scans/:id/duplicates?mode=name+size|name+size+mtime&limit=
GET /api/v1/scans/:id/diff/:otherId
GET /api/v1/scans/:id/export?format=csv|json&scope=
```

## Docker (read-only)

```
GET /api/v1/docker/status
GET /api/v1/docker/containers
GET /api/v1/docker/mounts
GET /api/v1/docker/stats
```

## Maintenance

```
POST /api/v1/gc        # { keep: number } — delete completed snapshots beyond the newest N
```

The wire types are camelCase and mirror `packages/api-types`.
