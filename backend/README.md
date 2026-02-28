# EdgeAI Blockchain

**The Most Intelligent Data Chain for Edge AI**

EdgeAI Blockchain 是一个专为边缘AI应用场景设计的区块链系统，采用创新的 **PoIE (Proof of Information Entropy)** 共识机制，直接评估和奖励高质量数据贡献。

## 🌟 核心特性

### 1. PoIE 共识机制 (信息熵证明)
- 基于数据信息熵评估数据质量
- 高质量数据贡献者获得更多奖励
- 自动检测和惩罚重复/低质量数据
- 公平的验证者选择算法

### 2. 区块链核心功能
- 完整的区块和交易系统
- 账户余额管理
- 多种交易类型支持
- 链验证和完整性检查

### 3. 智能合约
- **数据市场合约**: 数据上架、购买、评价
- **联邦学习合约**: 任务创建、参与者管理、模型更新
- **设备注册合约**: IoT设备注册和贡献记录

### 4. 数据市场
- 数据上架和定价
- 基于质量评分的搜索和排序
- 购买记录和评价系统
- 多种数据类别支持

### 5. P2P 网络
- 节点发现和连接管理
- 消息广播和同步
- 多种节点类型支持

### 6. 区块链浏览器
- 实时区块和交易查看
- 账户余额查询
- 数据市场浏览
- 交易创建工具

## 🚀 快速开始

### 环境要求
- Rust 1.70+
- Linux/macOS/Windows

### 编译
```bash
cd edgeai-blockchain
cargo build --release
```

### 运行节点
```bash
./target/release/edgeai-node
```

节点启动后:
- API 端点: http://localhost:8080/api/
- 区块链浏览器: http://localhost:8080/

## 📡 API 接口

### 区块链
| 端点 | 方法 | 描述 |
|------|------|------|
| `/api/chain` | GET | 获取区块链信息 |
| `/api/blocks` | GET | 获取所有区块 |
| `/api/blocks/{index}` | GET | 获取指定区块 |
| `/api/blocks/latest` | GET | 获取最新区块 |

### 交易
| 端点 | 方法 | 描述 |
|------|------|------|
| `/api/transactions/{hash}` | GET | 获取交易详情 |
| `/api/transactions/pending` | GET | 获取待处理交易 |
| `/api/transactions/transfer` | POST | 创建转账交易 |
| `/api/transactions/contribute` | POST | 创建数据贡献交易 |

### 账户
| 端点 | 方法 | 描述 |
|------|------|------|
| `/api/accounts/{address}` | GET | 获取账户信息 |
| `/api/accounts/{address}/balance` | GET | 获取账户余额 |
| `/api/accounts/{address}/transactions` | GET | 获取账户交易 |

### 挖矿
| 端点 | 方法 | 描述 |
|------|------|------|
| `/api/mine` | POST | 挖掘新区块 |

### 共识
| 端点 | 方法 | 描述 |
|------|------|------|
| `/api/validators` | GET | 获取验证者列表 |
| `/api/validators/register` | POST | 注册验证者 |

### 数据市场
| 端点 | 方法 | 描述 |
|------|------|------|
| `/api/marketplace` | GET | 获取市场列表 |
| `/api/marketplace/stats` | GET | 获取市场统计 |
| `/api/marketplace/list` | POST | 上架数据 |
| `/api/marketplace/purchase` | POST | 购买数据 |
| `/api/marketplace/{hash}` | GET | 获取数据详情 |

### 网络
| 端点 | 方法 | 描述 |
|------|------|------|
| `/api/network` | GET | 获取网络状态 |
| `/api/network/peers` | GET | 获取节点列表 |

## 📁 项目结构

```
edgeai-blockchain/
├── Cargo.toml              # 项目配置
├── README.md               # 项目文档
├── src/
│   ├── main.rs             # 主程序入口
│   ├── blockchain/         # 区块链核心
│   │   ├── mod.rs
│   │   ├── block.rs        # 区块定义
│   │   ├── transaction.rs  # 交易定义
│   │   └── chain.rs        # 链管理
│   ├── consensus/          # 共识机制
│   │   ├── mod.rs
│   │   └── poie.rs         # PoIE 实现
│   ├── contracts/          # 智能合约
│   │   ├── mod.rs
│   │   └── smart_contract.rs
│   ├── data_market/        # 数据市场
│   │   ├── mod.rs
│   │   └── marketplace.rs
│   ├── network/            # P2P 网络
│   │   ├── mod.rs
│   │   └── p2p.rs
│   └── api/                # REST API
│       ├── mod.rs
│       └── rest.rs
├── static/                 # 区块链浏览器
│   └── index.html
└── tests/                  # 测试文件
```

## 🔧 API 使用示例

### 创建转账
```bash
curl -X POST http://localhost:8080/api/transactions/transfer \
  -H "Content-Type: application/json" \
  -d '{"from": "genesis", "to": "alice", "amount": 10000}'
```

### 贡献数据
```bash
curl -X POST http://localhost:8080/api/transactions/contribute \
  -H "Content-Type: application/json" \
  -d '{"sender": "node1", "data": "Temperature: 25.5C, Humidity: 60%"}'
```

### 挖掘区块
```bash
curl -X POST http://localhost:8080/api/mine \
  -H "Content-Type: application/json" \
  -d '{"validator": "miner1"}'
```

### 查询余额
```bash
curl http://localhost:8080/api/accounts/alice/balance
```

## 📊 PoIE 数据质量评估

PoIE 共识机制通过以下指标评估数据质量:

| 指标 | 权重 | 描述 |
|------|------|------|
| 信息熵 | 40% | 数据的信息密度 (0-8 bits) |
| 唯一性 | 20% | 数据的独特程度 |
| 新鲜度 | 20% | 数据的时效性 |
| 完整性 | 20% | 数据的完整程度 |

综合评分 = (熵/8 × 0.4) + (唯一性 × 0.2) + (新鲜度 × 0.2) + (完整性 × 0.2)

## 🔐 安全特性

- SHA-256 哈希算法
- Ed25519 数字签名
- Merkle 树验证
- 重复数据检测
- 验证者惩罚机制

## 📜 许可证

MIT License

## 🤝 贡献

欢迎提交 Issue 和 Pull Request!

---

**EdgeAI Blockchain** - 让边缘AI数据更有价值

