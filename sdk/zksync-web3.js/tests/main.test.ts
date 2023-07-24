import { ethers } from 'ethers';

import { Wallet, Provider, utils } from "../build/src";

const MNEMONIC="fine music test violin matrix prize squirrel panther purchase material script deal"
const DERIVE_PATH = "m/44'/60'/0'/0/1";

const WETH_ADDRESS = "0x8D4F22C9f1F22Ea745C7a828391Ef0E8D1692D4a";

const printBalance = async (wallet: Wallet,tag: string) => {
    const wethBalance = await wallet.getBalanceL1(WETH_ADDRESS);
    console.log(`[${tag}]WETH balance L1: `, ethers.utils.formatEther(wethBalance));

    let balance = await wallet.getBalance();
    console.log(`[${tag}]ETH balance L2: `, ethers.utils.formatEther(balance));
}

const test = async () => {

    let zkWallet = Wallet.fromMnemonic(MNEMONIC, DERIVE_PATH);
    zkWallet = zkWallet.connect(new Provider("http://10.202.3.175:3050"));
    zkWallet = zkWallet.connectToL1(new ethers.providers.JsonRpcProvider("https://rpc.sepolia.org"));

    const fee = await zkWallet.getFullRequiredDepositFee({
        token: utils.ETH_ADDRESS,
        to: zkWallet.address,
    })

    console.log(fee);

    await printBalance(zkWallet, "before deposit")

    const tx = await zkWallet.deposit({
        token: utils.ETH_ADDRESS,
        amount: ethers.utils.parseEther("10"),
        overrides: {
            gasLimit: 1000000,
        }
    });
    await tx.waitFinalize();

    await printBalance(zkWallet, "after deposit")


    const withdrawTx = await zkWallet.withdraw({
        token: utils.ETH_ADDRESS,
        amount: ethers.utils.parseEther("10"),
        overrides: {
            gasLimit: 1000000,
        }
    })
    const receipt = await withdrawTx.waitFinalize();
    
    const finalizeWithdrawTx = await zkWallet.finalizeWithdrawal(receipt.transactionHash)

    await finalizeWithdrawTx.wait();

    await printBalance(zkWallet, "after withdraw")
}

test();


