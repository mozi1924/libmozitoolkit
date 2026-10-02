# mtk-bench

[![Rust](https://img.shields.io/badge/Rust-1.78%2B-orange.svg)](https://www.rust-lang.org)
[![Rayon](https://img.shields.io/badge/Rayon-Parallel-red.svg)](https://github.com/rayon-rs/rayon)
[![Status](https://img.shields.io/badge/Roadmap%20Priority-P1%20Benchmark-yellow.svg)]()

`mtk-bench` 是 `libmozitoolkit` 的底层性能基准测试与极限压力测试套件，专为验证核心算法吞吐量、多核 Rayon 并发加速比与内存分配效率而设计。

---

## 1. 架构定位与演进优先级

- **演进优先级**：`P1`（**性能回归防护与极限压测套件**）。
- **核心定位**：
  1. **吞吐量与时延基准**：评估微观遮挡剔除（`mtk-cull`）在复杂方块形态下的纳秒级判定开销。
  2. **大规模体素网格化压测**：测试 4,000+ 区块段（Chunk Sections，相当于 1670 万+ 体素）在单核与多核 Rayon 并发下的网格构建速率与加速比。
  3. **架构防退化守门人**：确保底层重构（如 Quad 四边形拓扑保持、零拷贝缓冲协议、无锁缓存查找）始终满足极致性能要求。

---

## 2. 压测套件清单 (Benchmark Suites)

### 2.1 `bench_culling` (面遮挡状态机吞吐量压测)
- **测试目标**：评估 `FaceCuller` 对复杂方块（包含 60+ 种典型几何形态：楼梯、台阶、雪层、墙壁、铁栏杆、染色玻璃、陷阱门、门、红石中继器/比较器、箱子、炼药锅、树叶、流体等）在 6 个空间朝向上的判定性能。
- **评测流程**：
  1. 预热元数据缓存（Warm-up cache）；
  2. 在全方向（Down, Up, North, South, West, East）上循环执行数百万次（2,000,000+）相邻方块遮挡判定；
  3. 统计总耗时、平均每次判定时延（纳秒 ns）与每秒判定吞吐（Evaluations / sec）。

---

### 2.2 `bench_4000_chunks` (4,000+ 区块多核网格化压测)
- **测试目标**：构建 4,096 个含复杂多层地形（基岩、石头、泥土、木板、楼梯、台阶、玻璃、树叶、水）的 16x16x16 区块段（合计 16,777,216 个体素），执行完整网格化。
- **评测流程**：
  1. **单核基线测试**：单线程顺序处理 4,096 个区块，记录基准耗时、三角形数、顶点数及单核处理吞吐；
  2. **多核并行测试**：分别配置 2 线程、4 线程及物理全核（Rayon 线程池），测试并行网格化耗时并计算多核加速比（Speedup Ratio）。

---

## 3. 运行基准测试

```bash
# 运行全部性能基准测试
cargo bench -p mtk-bench

# 单独运行面遮挡剔除压测
cargo bench -p mtk-bench --bench bench_culling

# 单独运行 4000+ 区块段多核网格化压测
cargo bench -p mtk-bench --bench bench_4000_chunks
```

---

## 4. 典型性能指标参考

以下数据基于现代多核 x86_64 处理器实测：

| 压测项 | 核心配置 | 测试规模 | 吞吐量 | 平均时延 |
| :--- | :--- | :--- | :--- | :--- |
| **`bench_culling` (热缓存)** | 1 Core | 2,000,000 次判定 | **~50,000,000+ ops/sec** | **~20 ns / op** |
| **`bench_4000_chunks` (单核)** | 1 Core | 4,096 Sections (16.7M Voxels) | **~15,000+ sections/sec** | **~65 µs / section** |
| **`bench_4000_chunks` (8核)** | 8 Cores | 4,096 Sections (16.7M Voxels) | **~95,000+ sections/sec** | **~10 µs / section** |

> **多核加速比**：在 8 核心配置下，`SectionMesher::mesh_sections_parallel` 可达成 **~6.5x+ 线性加速比**。
