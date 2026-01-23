# spectro-rs (中文版)

[![Rust](https://img.shields.io/badge/language-Rust-orange.svg?style=flat-square)](https://www.rust-lang.org)
[![License](https://img.shields.io/badge/license-GPLv3-blue.svg?style=flat-square)](https://www.gnu.org/licenses/gpl-3.0)
[![Crates.io](https://img.shields.io/crates/v/spectro-core.svg?style=flat-square)](https://crates.io/crates/spectro-core)
[![Docs.rs](https://docs.rs/spectro-core/badge.svg?style=flat-square)](https://docs.rs/spectro-core)
[![Build Status](https://github.com/Tinnci/spectro-rs/actions/workflows/ci.yml/badge.svg?style=flat-square)](https://github.com/Tinnci/spectro-rs/actions)

[English Version](./README.md)

`spectro-rs` 是针对 X-Rite ColorMunki 系列光谱仪的 Rust 高性能驱动实现与测量算法库。本项目旨在提供安全、低延迟的硬件交互接口，支持跨平台（Windows, macOS, Linux）的精密色彩测量、显示器校准及环境光分析。

---

## 目录
- [核心功能](#核心功能)
- [快速开始](#快速开始)
- [操作指南](#操作指南)
- [技术指标](#技术指标)
- [高精度光谱算法](#高精度光谱算法)
- [项目架构](#项目架构)
- [维护与开发](#维护与开发)
- [开源协议](#开源协议)

---

## 核心功能

**跨平台支持**：基于 Rust 的内存安全特性，通过原生 USB 通信协议实现对主流操作系统的全兼容支持。内置**原生 CJK 字体深度优化**，自动调用系统字体（苹方、微软雅黑、Noto Sans CJK）确保完美排版。

**多模式测量**：
- **反射模式 (Reflective)**：集成自动化的 Dark-current（暗电流）与 White-tile（白板）参考校准。
- **发射模式 (Emissive)**：采用针对显示设备优化的光谱转换矩阵，确保高精度的色彩特征提取。
- **环境光模式 (Ambient)**：支持通过集成的扩散罩进行 Spectral Power Distribution (SPD) 采集。

**精密色度学引擎**：
- **实时计算**：确定性地计算 CIE XYZ, Chromaticity (x, y) 以及 CIE L*a*b* 坐标。
- **参数关联**：自动推导 Correlated Color Temperature (CCT) 与 Spectral Centroid。
- **标准遵循**：支持工业标准的常用光源 (D65, D50, A, F-series) 与观察者函数 (CIE 2°, 10°)。

---

## 快速开始

### 编译环境
本项目的构建依赖于 [Rust toolchain](https://rust-lang.org)（建议使用 Stable 或 Nightly 版本）。

### 构建与运行
通过 Git 部署并利用 Cargo 工作区进行模块化构建：

```bash
git clone https://github.com/Tinnci/spectro-rs.git
cd spectro-rs
```

**命令行接口 (CLI)**：适用于自动化脚本与无界面测量环境。
```bash
cargo run -p spectro-core
```

**图形界面 (GUI)**：提供基于实时光谱分析的交互式操作套件。
```bash
cargo run -p spectro-gui
```

*说明：在 Windows 操作系统中，若硬件未被正确识别，需通过 [Zadig](https://zadig.akeo.ie/) 将驱动程序手动替换为 `WinUSB`。*

---

## 操作指南

### 校准协议
为确保测量结果的科学性，每次测量前必须执行 **Restart Calibration** 序列：
1. 将设备拨盘切换至 **参考位置 (White Dot / Position 2)**。
2. 驱动程序将依次执行两阶段校准：Dark Frame 采集（建立传感器底噪基准）与 White Tile 归一化。

### 测量执行
- **屏幕发射模式**：拨盘切换至 **Position 4**，并将设备贴紧目标测量区域。
- **环境光模式**：拨盘切换至 **Position 1** 并确保扩散罩处于工作位置。

---

## 技术指标

本项目的核心逻辑衍生自 **ArgyllCMS** 开源项目，针对 Rust 所有权模型进行了二次优化：
- **EEPROM 序列化**：实现了对硬件内置多项式线性化参数及出厂校准矩阵的完整解析。
- **光谱映射**：将 128 个原始传感器通道高精度转置为 36 个标准的 10nm 光谱带（覆盖 380nm 至 730nm）。
- **性能优化**：利用 Zero-cost abstractions 确保高频连续测量时的极低 CPU 占用。
- **基准精度**：在标准白板上可实现参考级的白色点精度（如 $L^* > 96$, $a^* \approx 0$, $b^* \approx 0$）。

---

## 高精度光谱算法

`spectro-rs` 内置了一套定制的数字信号处理 (DSP) 算法，专门针对老旧硬件恢复与科研级精度进行了深度优化：

- **硬件级过采样 (Oversampling)**：实现了 8x 多帧硬件平均采集，有效抑制散粒噪声（Shot Noise）与电子抖动，将信噪比 (SNR) 提升了约 2.8 倍。
- **高动态范围 (HDR) 曝光**：强制执行高计数目标值 (10,500+) 与最小积分时间 (0.015s)，最大限度榨取 UV/蓝光与红光等低感光波段的有效信号。
- **信号门控动态外推 (Signal-Gated Extrapolation)**：基于动态峰值阈值 (25%) 自动检测传感器有效波段，通过掩蔽噪声盲区防止色彩失真（如“绿色残留”）。
- **稳健锚点选择**：采用先进的“内漂移”锚点选择逻辑（如 `first + 1` 策略），避开 LED 发射光谱的不稳定斜坡，彻底消除错误的黄/蓝色偏移。
- **光谱平滑处理**：算法流水线集成 3 点 Boxcar 滤波器 `[0.25, 0.5, 0.25]`，在确保光谱细节的前提下提供极高的色度学稳定性与重复性。
- **物理模型线性化**：基于稳健的死区 (Dead-zone) 与饱和度模型修正传感器非线性，确保全量程测量的一致性。

---

## 项目架构

- **`crates/spectro-core`**：底层驱动核心库。负责 USB I/O 调度、EEPROM 数据处理及基础数学引擎。
- **`crates/spectro-gui`**：基于 `egui` 框架构建的前端实现，提供实时数据可视化服务。

---

## 维护与开发

### CI/CD 流程
通过 GitHub Actions 实现持续集成，对 `main` 分支的每次提交执行严格的静态分析 (`clippy`) 与单元测试。发布流程遵循语义化版本规范。

### 协作规范
维护者需在本地部署 Pre-commit 钩子，以确保代码风格与项目规范保持严格一致：
```bash
pre-commit install
```

---

## 开源协议

本项目根据 **[GNU General Public License v3.0](https://www.gnu.org/licenses/gpl-3.0.html)** 协议条款发布。

---

## 维护者

当前由核心开发团队进行维护。如需报告 Bug 或提交针对新硬件的支持申请，请查阅 GitHub Issue 追踪系统。
