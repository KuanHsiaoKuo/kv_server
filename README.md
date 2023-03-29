# kv_server

> 这是一个系列练手项目，包含若干个升级版本。

## Q&A

1. Blocking waiting for file lock on package cache

```shell
rm ~/.cargo/.package-cache
```

2. 基本运行指令可以在终端执行，IDEA等IDE的更新有问题。尤其是更新引入的包名之后。

> 可以在idea中将项目关闭，然后删除.idea配置文件，接着重新打开就可以马上修复。

## examples:

> 经过重构，将之前7个版本的迭代放在examples中，以最后一个版本作为最后实现

### 一、basic_kv_service

- [基本流程 - Anatomy In First Rust Programming Class 🦀](https://kuanhsiaokuo.github.io/geektime-tyr-rust/kv1_basic.html)
- [实现并验证协议层 - Anatomy In First Rust Programming Class 🦀](https://kuanhsiaokuo.github.io/geektime-tyr-rust/kv2_protocols.html)

1. 注意rust的代码组织方式：Workspace -> Package -> Crate -> Module(Diretory/File) -> mod -> Items
2. lib.rs、main.rs、mod.rs这三种特殊文件在cargo中的作用
3. rust代码的两种执行方式: rustc和cargo xxx

### 二、trait_kv_service

- [高级trait改造 - Anatomy In First Rust Programming Class 🦀](https://kuanhsiaokuo.github.io/geektime-tyr-rust/kv3_advanced_traits.html)

### 三、frame_kv_service

- [网络处理 - Anatomy In First Rust Programming Class 🦀](https://kuanhsiaokuo.github.io/geektime-tyr-rust/kv4_network.html)o

## 四、tls_kv_server

- [网络安全 - Anatomy In First Rust Programming Class 🦀](https://kuanhsiaokuo.github.io/geektime-tyr-rust/kv5_network_security.html)

## 五、async_kv_server

- [异步改造 - Anatomy In First Rust Programming Class 🦀](https://kuanhsiaokuo.github.io/geektime-tyr-rust/kv6_async_refactor.html)

## 六、pub_sub_kv_server

- [重大重构 - Anatomy In First Rust Programming Class 🦀](https://kuanhsiaokuo.github.io/geektime-tyr-rust/kv7_big_refactor.html)

## 七、prod_kv_server

- [配置、测试、监控、CI/CD - Anatomy In First Rust Programming Class 🦀](https://kuanhsiaokuo.github.io/geektime-tyr-rust/kv8_config_ci_cd.html)

## 生产代码梳理