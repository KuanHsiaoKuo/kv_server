# kv_server

> 这是一个系列练手项目，包含若干个升级版本。

## Q&A

1. Blocking waiting for file lock on package cache

```shell
rm ~/.cargo/.package-cache
```

2. 基本运行指令可以在终端执行，IDEA等IDE的更新有问题。尤其是更新引入的包名之后。
> 可以在idea中将项目关闭，然后删除.idea配置文件，接着重新打开就可以马上修复。

## 一、basic_kv_service
1. 注意rust的代码组织方式：Workspace -> Package -> Crate -> Module(Diretory/File) -> mod -> Items
2. lib.rs、main.rs、mod.rs这三种特殊文件在cargo中的作用
3. rust代码的两种执行方式: rustc和cargo xxx
