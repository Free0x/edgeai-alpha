const { ethers } = require("hardhat");

async function main() {
  const [deployer] = await ethers.getSigners();
  
  console.log("Deployer address:", deployer.address);
  
  const balance = await ethers.provider.getBalance(deployer.address);
  console.log("Balance:", ethers.formatEther(balance), "BNB");
  
  if (balance < ethers.parseEther("0.01")) {
    console.log("\n⚠️ Warning: Balance is low. You may need more tBNB for deployment.");
    console.log("Get tBNB from: https://testnet.bnbchain.org/faucet-smart");
  } else {
    console.log("\n✅ Balance is sufficient for deployment.");
  }
}

main()
  .then(() => process.exit(0))
  .catch((error) => {
    console.error(error);
    process.exit(1);
  });
