# EdgeAI 跨链桥部署与测试指南

## 概述

EdgeAI 跨链桥实现了 EdgeAI 原生链与 BSC（币安智能链）之间的代币互通。用户可以将 EDGE 代币桥接到 BSC 上成为 wEDGE（Wrapped EDGE），从而在 BSC 生态中使用，包括在 PancakeSwap 等 DEX 上交易。

## 部署信息

### 合约地址（BSC Testnet）

| 合约 | 地址 | 用途 |
|------|------|------|
| **WrappedEDGE (wEDGE)** | `0xEe3131549D8727bBCd6e628D90D6b57cf99F5794` | ERC-20 代币合约，代表桥接后的 EDGE |
| **EdgeAIBridge** | `0x0f72c1d37F64f0E962278A1941EC7664D4e2289B` | 跨链桥合约，处理锁定/释放逻辑 |

### 网络配置

| 参数 | 值 |
|------|-----|
| **网络名称** | BSC Testnet |
| **Chain ID** | 97 (0x61) |
| **RPC URL** | https://data-seed-prebsc-1-s1.binance.org:8545 |
| **区块浏览器** | https://testnet.bscscan.com |
| **原生代币** | tBNB |

### 部署者信息

| 参数 | 值 |
|------|-----|
| **部署者地址** | 0x562F1ed46e86B5d5b5d91569372d4c4cE4341336 |
| **部署时间** | 2026-01-14 |
| **wEDGE 最大供应量** | 1,000,000,000 EDGE |

---

## 架构说明

### 跨链桥工作流程

```
┌─────────────────────────────────────────────────────────────────┐
│                    EdgeAI Chain → BSC (锁定铸造)                  │
├─────────────────────────────────────────────────────────────────┤
│  1. 用户在 EdgeAI 链上发起桥接请求                                  │
│  2. EdgeAI 链锁定用户的 EDGE 代币                                  │
│  3. 桥接中继器监听锁定事件                                         │
│  4. 中继器调用 BSC 上的 Bridge 合约铸造等量 wEDGE                   │
│  5. wEDGE 发送到用户的 EVM 地址                                    │
└─────────────────────────────────────────────────────────────────┘

┌─────────────────────────────────────────────────────────────────┐
│                    BSC → EdgeAI Chain (销毁释放)                  │
├─────────────────────────────────────────────────────────────────┤
│  1. 用户在 BSC 上调用 Bridge 合约的 bridgeToEdgeAI 函数            │
│  2. Bridge 合约销毁用户的 wEDGE                                    │
│  3. 桥接中继器监听销毁事件                                         │
│  4. 中继器在 EdgeAI 链上释放等量 EDGE 到用户地址                    │
└─────────────────────────────────────────────────────────────────┘
```

### 合约角色

| 角色 | 权限 | 持有者 |
|------|------|--------|
| **DEFAULT_ADMIN_ROLE** | 管理所有角色 | 部署者 |
| **BRIDGE_ROLE** | 铸造/销毁 wEDGE | EdgeAIBridge 合约 |
| **PAUSER_ROLE** | 暂停/恢复合约 | 部署者 |

---

## 部署步骤

### 前置条件

1. Node.js 18+ 和 pnpm
2. BSC 测试网钱包，需有 tBNB（从 [BSC Faucet](https://testnet.bnbchain.org/faucet-smart) 获取）
3. 私钥配置

### 1. 环境配置

```bash
cd bridge/contracts
pnpm install

# 创建 .env 文件
echo "PRIVATE_KEY=0x你的私钥" > .env
```

### 2. 检查余额

```bash
npx hardhat run scripts/checkBalance.js --network bscTestnet
```

预期输出：
```
Deployer address: 0x...
Balance: 10.0 BNB
✅ Balance is sufficient for deployment.
```

### 3. 部署合约

```bash
npx hardhat run scripts/deploy.js --network bscTestnet
```

预期输出：
```
Deploying contracts with the account: 0x...
1. Deploying WrappedEDGE (wEDGE)...
   WrappedEDGE deployed to: 0x...
2. Deploying EdgeAIBridge...
   EdgeAIBridge deployed to: 0x...
3. Granting BRIDGE_ROLE to EdgeAIBridge...
   BRIDGE_ROLE granted to bridge contract
========================================
Deployment Summary
========================================
Network: bscTestnet
WrappedEDGE (wEDGE): 0x...
EdgeAIBridge: 0x...
========================================
```

### 4. 验证合约（可选）

```bash
npx hardhat verify --network bscTestnet 0xEe3131549D8727bBCd6e628D90D6b57cf99F5794 "0x部署者地址"
npx hardhat verify --network bscTestnet 0x0f72c1d37F64f0E962278A1941EC7664D4e2289B "wEDGE地址" "admin地址" "fee地址"
```

---

## 测试方案

### 测试环境准备

| 组件 | 地址/URL |
|------|----------|
| **EdgeAI 后端** | https://edgeai-blockchain-node.fly.dev |
| **EdgeAI 前端** | https://edgeai-alpha.vercel.app |
| **BSC Testnet RPC** | https://data-seed-prebsc-1-s1.binance.org:8545 |

### 测试用例

#### 测试 1：EdgeAI → BSC 桥接（锁定铸造）

**步骤：**

1. **准备 EdgeAI 钱包**
   - 访问 https://edgeai-alpha.vercel.app/wallet
   - 创建或导入钱包
   - 从 Faucet 获取测试 EDGE（自动获得 1000 EDGE）

2. **准备 MetaMask**
   - 安装 MetaMask 浏览器扩展
   - 添加 BSC Testnet 网络：
     - 网络名称：BSC Testnet
     - RPC URL：https://data-seed-prebsc-1-s1.binance.org:8545
     - Chain ID：97
     - 符号：tBNB
     - 浏览器：https://testnet.bscscan.com

3. **添加 wEDGE 代币到 MetaMask**
   - 点击 "Import tokens"
   - 合约地址：`0xEe3131549D8727bBCd6e628D90D6b57cf99F5794`
   - 符号：wEDGE
   - 小数位：18

4. **执行桥接**
   - 访问 https://edgeai-alpha.vercel.app/bridge
   - 连接 MetaMask
   - 选择方向：EdgeAI → BSC
   - 输入金额（如 100 EDGE）
   - 确认交易

5. **验证结果**
   - 检查 EdgeAI 钱包余额减少
   - 检查 MetaMask 中 wEDGE 余额增加
   - 在 BSCScan 查看交易记录

**预期结果：**
- EdgeAI 余额：900 EDGE（减少 100）
- wEDGE 余额：100 wEDGE（增加 100）

#### 测试 2：BSC → EdgeAI 桥接（销毁释放）

**步骤：**

1. **确保有 wEDGE 余额**
   - 完成测试 1 或通过管理员铸造

2. **执行反向桥接**
   - 访问 Bridge 页面
   - 选择方向：BSC → EdgeAI
   - 输入金额（如 50 wEDGE）
   - 输入 EdgeAI 接收地址
   - 确认 MetaMask 交易

3. **验证结果**
   - 检查 wEDGE 余额减少
   - 检查 EdgeAI 钱包余额增加

**预期结果：**
- wEDGE 余额：50 wEDGE（减少 50）
- EdgeAI 余额：950 EDGE（增加 50）

#### 测试 3：桥接费用验证

**步骤：**

1. 调用 Bridge 合约的 `calculateFee` 函数
2. 验证费用计算正确（默认 0.1%）

```javascript
// 使用 ethers.js
const fee = await bridge.calculateFee(ethers.parseEther("1000"));
console.log("Fee for 1000 EDGE:", ethers.formatEther(fee)); // 应为 1 EDGE
```

#### 测试 4：暂停/恢复功能

**步骤：**

1. 使用管理员账户调用 `pause()`
2. 尝试桥接，应该失败
3. 调用 `unpause()` 恢复
4. 再次桥接，应该成功

### 自动化测试脚本

```javascript
// scripts/test-bridge.js
const { ethers } = require("hardhat");

async function main() {
  const [admin, user] = await ethers.getSigners();
  
  // 加载合约
  const wEDGE = await ethers.getContractAt("WrappedEDGE", "0xEe3131549D8727bBCd6e628D90D6b57cf99F5794");
  const bridge = await ethers.getContractAt("EdgeAIBridge", "0x0f72c1d37F64f0E962278A1941EC7664D4e2289B");
  
  console.log("=== Bridge Test Suite ===\n");
  
  // 测试 1: 检查初始状态
  console.log("Test 1: Check initial state");
  const stats = await bridge.getBridgeStats();
  console.log("  Total bridged to EVM:", ethers.formatEther(stats[0]));
  console.log("  Total bridged to EdgeAI:", ethers.formatEther(stats[1]));
  console.log("  ✅ Initial state verified\n");
  
  // 测试 2: 管理员铸造测试代币
  console.log("Test 2: Admin mint wEDGE");
  const mintAmount = ethers.parseEther("1000");
  await bridge.mintForBridge(user.address, mintAmount, "test_edgeai_tx_123", "edgeai_addr_123");
  const balance = await wEDGE.balanceOf(user.address);
  console.log("  User wEDGE balance:", ethers.formatEther(balance));
  console.log("  ✅ Mint successful\n");
  
  // 测试 3: 用户桥接回 EdgeAI
  console.log("Test 3: Bridge back to EdgeAI");
  const bridgeAmount = ethers.parseEther("100");
  await wEDGE.connect(user).approve(bridge.target, bridgeAmount);
  await bridge.connect(user).bridgeToEdgeAI(bridgeAmount, "edgeai_user_address");
  const newBalance = await wEDGE.balanceOf(user.address);
  console.log("  User wEDGE balance after bridge:", ethers.formatEther(newBalance));
  console.log("  ✅ Bridge to EdgeAI successful\n");
  
  console.log("=== All tests passed! ===");
}

main().catch(console.error);
```

运行测试：
```bash
npx hardhat run scripts/test-bridge.js --network bscTestnet
```

---

## 故障排除

### 常见问题

| 问题 | 原因 | 解决方案 |
|------|------|----------|
| 交易失败：insufficient funds | tBNB 不足 | 从 Faucet 获取更多 tBNB |
| 交易失败：execution reverted | 合约暂停或权限不足 | 检查合约状态和调用者权限 |
| MetaMask 不显示 wEDGE | 未添加代币 | 手动导入代币合约地址 |
| 桥接后余额未更新 | 中继器延迟 | 等待几分钟后刷新 |

### 日志查看

- **BSC 交易**：https://testnet.bscscan.com/address/0x0f72c1d37F64f0E962278A1941EC7664D4e2289B
- **EdgeAI API**：https://edgeai-blockchain-node.fly.dev/api/bridge/history

---

## 安全注意事项

1. **私钥保护**：永远不要在代码中硬编码私钥，使用环境变量
2. **合约验证**：在 BSCScan 上验证合约源码以提高透明度
3. **权限管理**：定期审查 BRIDGE_ROLE 持有者
4. **暂停机制**：发现异常时立即暂停合约
5. **费用设置**：根据市场情况调整桥接费用

---

## 相关链接

- **GitHub 仓库**：https://github.com/Free0x/edgeai-alpha
- **前端应用**：https://edgeai-alpha.vercel.app
- **后端 API**：https://edgeai-blockchain-node.fly.dev
- **SDK (npm)**：https://www.npmjs.com/package/@free0x/edgeai-sdk
- **BSCScan (wEDGE)**：https://testnet.bscscan.com/address/0xEe3131549D8727bBCd6e628D90D6b57cf99F5794
- **BSCScan (Bridge)**：https://testnet.bscscan.com/address/0x0f72c1d37F64f0E962278A1941EC7664D4e2289B
