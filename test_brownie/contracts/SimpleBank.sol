// SPDX-License-Identifier: MIT
pragma solidity >=0.6.6 <0.9.0;
import "interfaces/AggregatorV3Interface.sol";

contract SimpleBank {
  address owner;
  address[] public accounts;
  uint256 public usdEntryFee;
  AggregatorV3Interface public priceFeed;
  mapping(address => uint256) public balances;

  constructor(address _priceFeed) public {
    priceFeed = AggregatorV3Interface(_priceFeed);
    owner = msg.sender;
    balances[msg.sender] = 1000;
  }

  function transfer(address receiver, uint256 amt) public {
    require(balances[msg.sender] >= amt, "not enough balances");
    require(msg.sender != receiver, "not enough balances");
    balances[msg.sender] -= amt;
    balances[receiver] += amt;
  }

  function getListAccountAndBalaces() public view returns(address[] memory,uint256[] memory){
    uint256[] memory balanceOfAccount = new uint256[](accounts.length);
    for(uint256 i = 0 ; i < accounts.length ; i++){
      balanceOfAccount[i] = balances[accounts[i]];
    }
    return (accounts,balanceOfAccount);
  }
  function depositToSC() public payable {
    uint flag = 1;
    uint256 minimumUSD = 50 * 10**18;
    require(
      getConversionRate(msg.value) >= minimumUSD,
      "You need to donate more ETH!"
    );
    balances[msg.sender] += msg.value;
    // example
    // addressToAmountFunded[0xab] = 1 ETH
    // addressToAmountFunded[0xcd] = 1 ETH
    for (uint256 i = 0; i < accounts.length; i++) {
      address account = accounts[i];
      if(account == msg.sender){
        flag = 0;
      }
    }
    if(flag == 1){
      accounts.push(msg.sender);
    }
  }

  // 10^18 eth -> USD
  function getPrice() public view returns (uint256) {
    (, int256 answer, , , ) = priceFeed.latestRoundData();
    return uint256(answer * 10000000000);
    // 4259 912137930 000000000
  }

  function getConversionRate(uint256 ethAmouth) public view returns (uint256) {
    uint256 ethPrice = getPrice();
    uint256 ethAmountInUsd = (ethPrice * ethAmouth) / 1000000000000000000;
    return ethAmountInUsd;
  }

  // return EntraceFee (wei)
  function getEntranceFee() public view returns (uint256) {
    uint256 minimumUSD = 50 * 10**18;
    uint256 price = getPrice();
    uint256 precision = 10**18;
    return (minimumUSD * precision) / price;
  }

  modifier onlyOwner() {
    require(msg.sender == owner, "you're not owner");
    _;
  }

  function withdraw() public payable onlyOwner {
    payable(msg.sender).transfer(address(this).balance); // transfer from address(this) to msg.sender (address.this is this contract)
    for (uint256 i = 0; i < accounts.length; i++) {
      address account = accounts[i];
      balances[account] = 0;
    }
    accounts = new address[](0);
  }

}
