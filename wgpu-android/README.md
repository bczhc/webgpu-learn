wgpu Android demo
---

## Building Project

First create file `local.properties` writing this (modify for your local path) under project root:

```properties
sdk.dir=/home/bczhc/bin/AndroidSdk
ndk.dir=/mnt/nvme/AndroidSdk/ndk/25.1.8937393
```

Then create config file `config.toml` under project root:

```toml
[ndk]
targets = ["arm64-v8a-21"]
build_type = "release"
```

Then, do

```shell
./gradlew asD
```
