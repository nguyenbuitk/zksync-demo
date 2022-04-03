from brownie import SimpleBank, network, config, MockV3Aggregator
from scripts.helpful_scripts import get_account, deploy_mocks, LOCAL_ENVIRONMENTS
# mocks nhằm mục đích lấy price_feed_account

# dùng account(0) để deploy
def deploy_fund_me():
    account = get_account()
    print("account_address: ", account)
    if network.show_active() not in LOCAL_ENVIRONMENTS:
        price_feed_address = config["networks"][network.show_active()]["eth_usd_price_feed"]
    else:
        deploy_mocks()
        price_feed_address = MockV3Aggregator[-1].address

    fund_me = SimpleBank.deploy(price_feed_address, {"from": account}, publish_source= config["networks"][network.show_active()].get("verify"))
    print(f"Contract deployed to {fund_me.address}")
    return fund_me

def main():
    deploy_fund_me()
