# `proto/` is intentionally empty

This directory exists only as a placeholder.  There are no vendored `.proto`
files and there will not be — see the [workspace README](../README.md) for
the reasoning.

In short: the Starlink `.proto` files are not published by SpaceX, every
community mirror is either stale or ambiguously-licensed, and extracting
via reflection is only useful against a live dish.  Rather than plumb any
of that into the build, the `starlink-proto` crate hand-writes the request
encoders for the oneof arms we actually use and exposes an in-process
gRPC reflection client (`starlink-proto::reflection`) that can pull a
`FileDescriptorSet` off a live dish at runtime when we want to verify our
hand-written assumptions or pick up new fields.

If you want to dump the current schema from a dish for comparison, the
traditional one-liner is:

```sh
grpcurl -plaintext -protoset-out dish.protoset \
        192.168.100.1:9200 \
        describe SpaceX.API.Device.Device
```

Don't check the output into this directory — treat it as scratch data.
