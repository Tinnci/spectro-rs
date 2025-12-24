# 🌈 spectro-rs (中文版)

[English Version](./README.md)

**spectro-rs** 是一个基于 Rust 开发的高性能 X-Rite ColorMunki (Original/Design) 光谱仪驱动程序。

---

## ✨ 核心功能

- **🚀 跨平台支持**：Windows, macOS, Linux 通用。
- **📊 全模式测量**：
    - **反射模式 (Reflective)**：带自动白板校准。
    - **发射模式 (Emissive)**：专用显示器测量矩阵。
    - **环境光模式 (Ambient)**：支持环境光扩散罩。
- **🧪 色度学引擎**：
    - 实时计算 **XYZ**, **x,y 坐标** 和 **L*a*b***。
    - 自动估算 **CCT (色温)** 和 **质心波长**。
- **🎨 光谱可视化**：终端彩色柱状图展示光谱分布。

---

## 🛠️ 快速开始

### 1. 运行环境
- 安装 [Rust 编译环境](https://rust-lang.org)。
- **Windows 用户**：若无法识别，请用 [Zadig](https://zadig.akeo.ie/) 将驱动更换为 `WinUSB`。

### 2. 运行
```bash
cargo run
```

---

## 📖 操作建议

1. **校准**：测量前请在“白点”位执行 **Restart Calibration**。
2. **屏幕测量**：将拨盘转至测量位，选择 **Measure Emissive**。
3. **环境光测量**：将拨盘转至扩散罩位，选择 **Measure Ambient**。

---

## 🏛️ 技术说明

本项目深度参考了 **ArgyllCMS** 的核心算法逻辑：
- 实现了完整的 EEPROM 解析和多项式线性化。
- 支持 380nm - 730nm 的标准光谱映射。

---

## ⚖️ 授权协议

本项目采用 **[GPLv3](https://www.gnu.org/licenses/gpl-3.0.html)** 加密开源协议。

---

## 🤝 贡献计划

欢迎通过 Issue 或 PR 提交反馈！共同打造更强大的 Rust 色彩工具。
