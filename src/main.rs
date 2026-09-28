//! cowsay —— BORUIX 第三方程序样例（b3p 构建目标）。
//!
//! 本程序是 `sdk` 的 b3p（build third-party）子命令的验证载体，同时充当
//! 第三方开发者的最小可抄模板：它只依赖 `libsys`，导出 `user_main`，
//! 经真实 `int 0x80` 写标准输出，不带任何私有的内核特权。
//!
//! ## 与内置程序的区别
//!
//! 内置程序（init/shell/...）经 `USER_PROGRAMS` 编进 liveCD payload，
//! 被内核 `include_bytes!` 嵌入镜像。本程序**不进 payload**：它由 b3p
//! 编译后落到 `sdk/diskfiles/3p/`，随数据盘以 `/3p/cowsay.elf` 存在，
//! 由用户在 shell 里经 `/volumes/BORUIX_DATA/3p/cowsay.elf` 执行。
//!
//! 该路径成立的前提是内核 `sys_exec` 的 `_` 分支把 a1 当**纯 VFS 路径**
//! 解析（kernel syscall.rs:2917），而非「程序必须在 /programs 下」。
//!
//! ## 参数 ABI（这一条最容易被误解，故详解）
//!
//! BORUIX 的进程入口**不传拆分好的 argv**。权威定义见
//! `docs/abi/syscall-abi.md` §4（loader `setup_user_stack`）：
//!
//! ```text
//! stack_top - 0x200 .. : 命令行字符串区（整条 cmd，NUL 结尾）
//! stack_top - 0x220 + 0x00 : argc = 1
//! stack_top - 0x220 + 0x08 : argv[0] = 指向该命令行字符串
//! stack_top - 0x220 + 0x10 : argv[1] = NULL
//! rsp at entry             = stack_top - 0x220
//! ```
//!
//! 即 **argc 恒为 1**，argv[0] 是**整条命令行**；按空格拆词是**用户程序的职责**，
//! 内核只做搬运。这就是 `libsys::exec_path(path, cmd)` 的 `cmd` 是
//! `&[u8]` 字节串而非 `&[&str]` 的原因。
//!
//! ### 这条命令行里不含程序名
//!
//! 上一条容易被误读成「argv[0] 像 POSIX 那样是程序名」。**不是。**
//! 经 shell 执行时，shell 的 `exec_via_path`（shell/src/commands.rs:1957-1964）
//! 在拼 cmd 时用 `skip(1)` **跳过命令名**，只把剩余参数空格连接后下传：
//!
//! ```text
//! 用户输入 : cowsay.elf 114514
//! shell 传 : "114514"        <- 只有参数，没有程序名
//! ```
//!
//! 故本程序**不剥首词**，整条命令行就是要说的话。剥首词会把唯一参数吃掉。
//!
//! 本程序据此自行切词：见 [`split_args`]。
//!
//! ## 用法
//!
//! ```text
//! /volumes/BORUIX_DATA/3p/cowsay.elf hello world
//! ```
//!
//! 无参数时说明用法并返回 2（用法错误），不打印默认文案冒充成功。

#![no_std]
#![no_main]

use libsys::{STDERR, STDOUT, write};

/// 程序名。仅用于错误提示与用法文本，不参与任何特权判定。
const PROG: &str = "cowsay";

/// 牛身（ASCII 图）。原样常量，不做任何运行时生成。
const COW: &[u8] = b"        \\   ^__^\n         \\  (oo)\\_______\n            (__)\\       )\\/\\\n                ||----w |\n                ||     ||\n";

/// 气泡最大内容宽度（字符）。
///
/// 选择理由：终端默认宽度约 80 列，气泡两侧各占 2 列（`| ` 与 ` |`），
/// 留出余量使整行不超过 80。该值是**本程序自己的排版策略**，
/// 不是内核 ABI 的一部分，故定义在此而非 libsys。
///
/// **不变量**：实际渲染行的最长者是框线，占
/// `2（角）+ (BUBBLE_WIDTH + 2)（横线）` = `BUBBLE_WIDTH + 4` 列。
/// [`rule`] 的缓冲即按此尺寸开，故内容宽度上限必须由它反推：
/// 内容超 `BUBBLE_WIDTH` 时框线缓冲会溢出，因此下面那个判定不是
/// 「美观限制」而是**内存安全边界**，不得放宽。
const BUBBLE_WIDTH: usize = 74;

/// 气泡整行最大列数 = 框线长度。供 [`rule`] 定缓冲尺寸（S15 单点）。
const BUBBLE_MAX_LINE: usize = BUBBLE_WIDTH + 4;

/// 命令行长度上限（字节）。与内核 `CMD_BUF_BYTES` 同量级，仅作防御。
const MAX_CMDLINE: usize = 4096;

/// 拆词数上限（含首词）。超出即报错，不静默丢弃。
const MAX_WORDS: usize = 64;

/// best-effort 输出：单次写失败没有可恢复动作，故忽略结果。
/// 需要知道成败的路径（用法提示）用 [`emit`]。
fn put(seg: &[u8]) {
    let _ = write(STDOUT, seg);
}

/// 写一段并返回是否全部写出。失败不静默。
fn emit(fd: u64, seg: &[u8]) -> bool {
    matches!(write(fd, seg), Ok(n) if n == seg.len())
}

/// 打印用法到 fd 2 并返回用法错误码 2。
fn usage() -> i32 {
    emit(STDERR, b"usage: ");
    emit(STDERR, PROG.as_bytes());
    let _ = write(STDERR, b" <word>...\n");
    2
}

/// 写一行气泡框线：`<left> + '-'*width + <right>`。
///
/// `width` 是**横线数**。要与 `| ` + text + ` |` 的中间行等宽，
/// 调用方须传 `text_len + 2`（两侧各一个空格）；传 `text_len` 会让
/// 框线比内容短 4 格，气泡对不齐。
fn rule(left: u8, right: u8, width: usize) {
    // 缓冲 = 左右角各 1 + 横线 width；width 最大为 BUBBLE_WIDTH + 2，
    // 故缓冲取 BUBBLE_MAX_LINE（= BUBBLE_WIDTH + 4）刚好容纳最大行。
    // 调用方已由 BUBBLE_WIDTH 判定保证不会越界。
    let mut line = [0u8; BUBBLE_MAX_LINE];
    line[0] = left;
    let mut i = 1;
    while i <= width {
        line[i] = b'-';
        i += 1;
    }
    line[width + 1] = right;
    put(&line[..width + 2]);
    put(b"\n");
}

/// 按空格/Tab 切词，返回切出的片段数（写入 `out`）。
///
/// 连续空白视为一个分隔符；首尾空白忽略。切出的每个片段是命令行缓冲的
/// 子切片，故 `out` 的生命周期受 `line` 约束。
///
/// 返回 `None` 表示词数超过 `out` 容量——**不静默丢弃**多余的词，
/// 因为那会让用户以为「我的参数生效了」而实际没有。
fn split_args<'a>(line: &'a [u8], out: &mut [&'a [u8]]) -> Option<usize> {
    let mut n = 0usize;
    let mut i = 0usize;
    while i < line.len() {
        // 跳过分隔符（空格与 Tab）。
        while i < line.len() && (line[i] == b' ' || line[i] == b'\t') {
            i += 1;
        }
        if i >= line.len() {
            break;
        }
        let start = i;
        while i < line.len() && line[i] != b' ' && line[i] != b'\t' {
            i += 1;
        }
        if n >= out.len() {
            return None;
        }
        out[n] = &line[start..i];
        n += 1;
    }
    Some(n)
}

/// 把词列表按单空格拼进 `buf`，返回有效长度；容量不足返回 `None`。
///
/// **绝不静默截断**：截断的气泡与用户输入不符，属 S09 意义上的伪输出。
fn join_words(words: &[&[u8]], buf: &mut [u8]) -> Option<usize> {
    let mut n = 0usize;
    for (i, w) in words.iter().enumerate() {
        if i > 0 {
            if n >= buf.len() {
                return None;
            }
            buf[n] = b' ';
            n += 1;
        }
        if n + w.len() > buf.len() {
            return None;
        }
        buf[n..n + w.len()].copy_from_slice(w);
        n += w.len();
    }
    Some(n)
}

/// 从入口参数块取出整条命令行，返回其字节切片（去尾 NUL）。
///
/// # Safety
///
/// `argv` 必须是内核 loader 按 `docs/abi/syscall-abi.md` §4 布局的
/// 参数块：`argc == 1`、`argv[0]` 指向 NUL 结尾的命令行字符串。
unsafe fn cmdline<'a>(argc: isize, argv: *const *const u8) -> Option<&'a [u8]> {
    // 无命令行时 argc 槽为 0（ABI §4：rsp = stack_top-0x10）。
    if argc <= 0 || argv.is_null() {
        return None;
    }
    let p = unsafe { *argv };
    if p.is_null() {
        return None;
    }
    let mut l = 0usize;
    while l < MAX_CMDLINE && unsafe { *p.add(l) } != 0 {
        l += 1;
    }
    if l >= MAX_CMDLINE {
        return None;
    }
    Some(unsafe { core::slice::from_raw_parts(p, l) })
}

#[unsafe(no_mangle)]
pub extern "C" fn user_main(argc: isize, argv: *const *const u8) -> i32 {
    // 重要：BORUIX 入口的 argc 恒为 1，argv[0] 是**整条命令行**。
    // 见文件头「参数 ABI」与 docs/abi/syscall-abi.md §4。
    let line = match unsafe { cmdline(argc, argv) } {
        Some(l) => l,
        None => return usage(),
    };

    let mut words: [&[u8]; MAX_WORDS] = [b""; MAX_WORDS];
    let n = match split_args(line, &mut words) {
        Some(n) => n,
        None => {
            emit(STDERR, b"cowsay: too many arguments\n");
            return 2;
        }
    };

    // 命令行本身就是参数，不含程序名——不要剥首词。
    //
    // 依据：shell 的 exec_via_path（shell/src/commands.rs:1957-1964）在
    // 拼 cmd 时用 skip(1) 跳过命令名，只把剩余参数空格连接后传下
    // （exec_path(path, arg)）。故本程序收到的是 "114514" 而非
    // "cowsay.elf 114514"。剥首词会把唯一参数吃掉。
    //
    // 直接调本程序时（不经 shell）同理：调用方传什么就是什么。
    if n == 0 {
        return usage();
    }
    let text_words = &words[..n];

    let mut text = [0u8; MAX_CMDLINE];
    let text_len = match join_words(text_words, &mut text) {
        Some(l) => l,
        None => {
            emit(STDERR, b"cowsay: arguments too long\n");
            return 2;
        }
    };

    // 超宽即拒绝，不折行：折行需要词边界算法，本样例保持语义直白，
    // 宁可报错也不产出与输入不符的图。
    //
    // 这个判定同时是 rule() 的缓冲安全边界（见 BUBBLE_WIDTH 不变量），
    // 不是纯美观限制——不得为了「多显示几个字」放宽它。
    if text_len > BUBBLE_WIDTH {
        emit(STDERR, b"cowsay: text wider than bubble\n");
        return 2;
    }

    // 框线宽 = 内容宽 + 2（两侧空格），与中间行 `| ` + text + ` |` 对齐。
    rule(b'/', b'\\', text_len + 2);
    put(b"| ");
    put(&text[..text_len]);
    put(b" |\n");
    rule(b'\\', b'/', text_len + 2);
    put(COW);
    0
}

