# cowsay

**简体中文** | [English](#english)

BORUIX 的 **cowsay**——一头会说话的 ASCII 牛，同时也是**第三方程序开发样例**。

```
/volumes/BORUIX_DATA/3p/cowsay.elf hello world
```

输出：

```
/-----------------\
| hello world     |
\-----------------/
        \   ^__^
         \  (oo)\_______
            (__)\       )\/\
                ||----w |
                ||     ||
```

---

## 它的双重身份

**一、它是一个能用的程序。** 把命令行参数显示在气泡里，下方配 ASCII 牛。

**二、它是一份可抄的最小模板。** 这是它更重要的用途：想为 BORUIX 写第三方程序的开发者，
可以拿它当起点——它只依赖 `libsys`，导出一个 `user_main`，通过真实的系统调用写标准输出，
不使用任何内核特权。

## 与内置程序的区别

BORUIX 的程序分两类，这个区别对第三方开发者很关键：

| | 内置程序 | 本程序 |
| --- | --- | --- |
| 例子 | `init`、`shell` | `cowsay` |
| 位置 | 编译进系统镜像的 payload | 数据盘上的 `/3p/cowsay.elf` |
| 更新方式 | 需重新构建镜像 | 替换数据盘上的文件即可 |

本程序**不进系统镜像**。它由第三方构建流程编译后放到数据盘，用户在 shell 里按普通文件路径执行。
这证明了"程序不必住在系统目录里"——内核按纯路径解析可执行文件，不要求它位于某个固定位置。

## 参数 ABI：最容易误解的一点

**BORUIX 的进程入口不传拆分好的参数列表。** 这一点值得单独说明，因为它和 POSIX 的直觉不同：

- 入口处 `argc` **恒为 1**
- `argv[0]` 是**整条命令行字符串**，不是程序名
- **按空格拆词是用户程序自己的职责**，内核只负责搬运字节

而且经 shell 执行时，shell 传下来的字符串里**不含程序名**：

```
用户输入 : cowsay.elf 114514
shell 传 : "114514"        ← 只有参数，没有程序名
```

所以本程序**不剥掉第一个词**——整条命令行就是要说的话。如果按 POSIX 习惯剥掉首词，会把唯一
的参数吃掉，输出一个空气泡。

## 边界与错误处理

程序宁可报错，也不产出与输入不符的图：

| 情况 | 行为 |
| --- | --- |
| 没有参数 | 打印用法到标准错误，返回 2 |
| 参数过多（超过 64 个词） | 如实报错，不静默丢弃多余的词 |
| 文本超出气泡宽度 | 如实报错，**不折行** |
| 参数总长超限 | 如实报错，不截断 |

**为什么不折行**：折行需要词边界算法，本样例刻意保持语义直白。而且气泡宽度同时是内部缓冲区的
安全边界——放宽它会导致框线缓冲溢出，所以那条判定不是美观限制。

**为什么不截断**：截断的气泡与用户输入不符，那是一种伪输出，比直接报错糟糕。

## 气泡宽度

气泡内容宽度上限为 74 个字符。终端默认宽度约 80 列，气泡两侧各占 2 列，留出余量使整行不超过 80。

## 构建

本程序由 BORUIX 的第三方构建流程编译：

```bash
cargo build --release
```

产物部署到数据盘的 `/3p/cowsay.elf`。

## 文件结构

```
cowsay/
├── Cargo.toml    # 包定义，仅依赖 libsys
├── build.rs      # 注入链接脚本
├── linker.ld     # 用户态段布局
└── src/
    └── main.rs   # 程序本体
```

## 相关项目

- [`libsys`](https://github.com/BRX-Boruix/libsys) —— 用户态系统调用封装，本程序唯一依赖

## 许可

MIT License，版权归 Yang Borui 所有。详见 [LICENSE](LICENSE)。

---

# English

[简体中文](#cowsay) | **English**

**cowsay** for BORUIX — an ASCII cow that speaks, and simultaneously a **third-party program
example**.

```
/volumes/BORUIX_DATA/3p/cowsay.elf hello world
```

Output:

```
/-----------------\
| hello world     |
\-----------------/
        \   ^__^
         \  (oo)\_______
            (__)\       )\/\
                ||----w |
                ||     ||
```

---

## Two roles in one

**First, it is a working program.** It displays its command-line arguments in a speech bubble above
an ASCII cow.

**Second, and more importantly, it is a minimal template to copy.** A developer who wants to write a
third-party program for BORUIX can start here: it depends only on `libsys`, exports a single
`user_main`, writes to standard output through real system calls, and uses no kernel privileges.

## How it differs from built-in programs

BORUIX has two kinds of programs, and the distinction matters to third-party developers:

| | Built-in | This program |
| --- | --- | --- |
| Examples | `init`, `shell` | `cowsay` |
| Location | compiled into the system image payload | `/3p/cowsay.elf` on the data disk |
| Updating | requires rebuilding the image | just replace the file on the data disk |

This program is **not part of the system image**. The third-party build produces it, it is placed on
the data disk, and the user runs it from the shell by ordinary file path. That demonstrates programs
need not live in a system directory — the kernel resolves the executable by plain path and does not
require it to sit in a fixed location.

## The argument ABI: the easiest thing to get wrong

**BORUIX's process entry point does not pass a split argument list.** This deserves its own section
because it differs from the POSIX intuition:

- `argc` at entry is **always 1**
- `argv[0]` is the **entire command-line string**, not the program name
- **Splitting it on spaces is the program's own job**; the kernel only moves bytes

Moreover, when launched through the shell, the string handed down **does not include the program
name**:

```
user types : cowsay.elf 114514
shell sends: "114514"        <- arguments only, no program name
```

So this program does **not** strip the first word — the whole command line is what should be said.
Stripping it out of POSIX habit would swallow the only argument and print an empty bubble.

## Edge cases and error handling

The program would rather fail than print a picture that does not match its input:

| Situation | Behaviour |
| --- | --- |
| No arguments | Prints usage to standard error, returns 2 |
| Too many arguments (over 64 words) | Reports honestly; never silently drops extra words |
| Text wider than the bubble | Reports honestly; **does not wrap** |
| Total argument length over the limit | Reports honestly; never truncates |

**Why it does not wrap**: wrapping needs a word-boundary algorithm, and this example deliberately
keeps its semantics plain. The bubble width is also the safety bound of an internal buffer —
loosening it would overflow the border buffer, so that check is not a cosmetic limit.

**Why it does not truncate**: a truncated bubble does not match the user's input, which is a form of
fake output — worse than an outright error.

## Bubble width

Bubble content is limited to 74 characters wide. A terminal is about 80 columns by default, the
bubble takes 2 columns on each side, leaving margin so no line exceeds 80.

## Building

The program is compiled by BORUIX's third-party build path:

```bash
cargo build --release
```

The artifact is deployed to `/3p/cowsay.elf` on the data disk.

## Layout

```
cowsay/
├── Cargo.toml    # package definition, depending only on libsys
├── build.rs      # injects the linker script
├── linker.ld     # user-space section layout
└── src/
    └── main.rs   # the program itself
```

## Related projects

- [`libsys`](https://github.com/BRX-Boruix/libsys) — the user-space syscall wrapper, this program's only dependency

## License

MIT License, copyright Yang Borui. See [LICENSE](LICENSE).
