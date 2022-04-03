from brownie import SimpleBank, accounts
from brownie.network import account
from scripts.helpful_scripts import get_account
from web3 import Web3
from hexbytes import HexBytes
import binascii
# deposit from account
def deposit(acc, fee):
    simple_bank = SimpleBank[-1]
    account = get_account()
    entrance_fee = simple_bank.getEntranceFee()
    print("Depositing ...")
    tx_hash = simple_bank.depositToSC({"from": acc, "value": fee})          # create transaction
    return tx_hash

def withdraw():
    simple_bank = SimpleBank[-1]
    account = get_account()
    simple_bank.withdraw({"from": account})                                 # create transaction

def getListAccountAndBalance():
    simple_bank = SimpleBank[-1]
    accountList, balanceList = simple_bank.getListAccountAndBalaces()
    ethToUSD = simple_bank.getPrice()/10**18

    for i in range (len(accountList)):
        print(f"Balances of: {accountList[i]} in SC: {balanceList[i]/(10**18)} ETH ~ {(balanceList[i]/(10**18))*ethToUSD} USD")

def main():
    w3 = Web3(Web3.HTTPProvider("HTTP://127.0.0.1:7545"))
    accA = get_account(index=0)
    accB = get_account(index=1)
    simple_bank = SimpleBank[-1]
    entrance_fee = simple_bank.getEntranceFee()
    entrance_fee_to_usd = (entrance_fee/10**18)*(simple_bank.getPrice()/10**18)
    # print(f"Entrace_fee is: {entrance_fee} Wei = {entrance_fee/10**18} ETH = {entrance_fee_to_usd} USD")
    # print(f"Price: 1 ETH = {simple_bank.getPrice()/10**18} USD")

    deposit(accA, entrance_fee*4)
    block = w3.eth.get_block('latest')
    # 'transactions': [HexBytes('0x57cb81d66b6acdcd01d3d956ff9a0a9946b164e5c8d5f35a8ac22160eb594664')]
    # 'hash': HexBytes('0xc539eb3886cbd0de0de535bfe55569cd951ff3143075f00a156e92b15174919f')
    # tx.hash =  b'\xcd\xd5P\x86\xc1\xf9\x07\x8ex\x99\x98\xe4\x04\x124\x1a\xd2\t\x03\xe4\x14\xcfMA\xf24\xd8s\x1c\x82\xe2H'
    tx_hash = block.transactions[0].hex()
    block_hash = block.hash.hex()
    print(f"account {accA} have deposited!")
    print("transaction_hash is: ", tx_hash)
    print("block_hash is: ", block_hash)
    print("============================================================================")

    deposit(accB, entrance_fee*7)
    tx_hash = block.transactions[0].hex()
    block_hash = block.hash.hex()
    print(f"account {accB} have deposited!")
    print("tx_hash = ", tx_hash)
    print("block_hash = ", block_hash)
    print("============================================================================")

    #withdraw()
    print("Infomation of account on Smart Contract")
    getListAccountAndBalance()
