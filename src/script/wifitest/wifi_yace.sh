#!/bin/bash

for i in {1..100}
do
    echo "第 $i 次执行"

    # 发送第一个配置
    response1=$(curl -s -X POST -d '{
        "w24": {
            "Switch": "1",
            "Password": "88888888",
            "SSID": "apv2_test_joker",
            "AuthMode": "4",
            "Channel": "0",
            "AutoChannel": "1",
            "TxPower": "100",
            "Hidden": "0",
            "BandWidth": "0",
            "Mode": "9"
        },
        "w5g": {
            "Switch": "1",
            "Password": "88888888",
            "SSID": "apv5_test_joker",
            "AuthMode": "4",
            "Channel": "0",
            "AutoChannel": "1",
            "TxPower": "100",
            "Hidden": "0",
            "BandWidth": "0",
            "Mode": "14"
        },
        "white_enable": false
    }' -H 'Content-Type: application/json' -k https://10.11.173.193:9900/api/cluster/devices/3e:ad:54:e1:13:4e/rpc/sysctrl/wifi/config_adv)
    echo '发送配置 1'
    echo "时间：$(date '+%Y-%m-%d %H:%M:%S')"
    echo "返回结果：$response1"
    echo '等待 100 秒'
    sleep 100
    

    # 发送第二个配置
    response2=$(curl -s -X POST -d '{
        "w24": {
            "Switch": "1",
            "Password": "123456789z",
            "SSID": "apv2_test_joker",
            "AuthMode": "4",
            "Channel": "0",
            "AutoChannel": "1",
            "TxPower": "100",
            "Hidden": "0",
            "BandWidth": "0",
            "Mode": "9"
        },
        "w5g": {
            "Switch": "1",
            "Password": "123456789z",
            "SSID": "apv5_test_joker",
            "AuthMode": "4",
            "Channel": "0",
            "AutoChannel": "1",
            "TxPower": "100",
            "Hidden": "0",
            "BandWidth": "0",
            "Mode": "14"
        },
        "white_enable": false,
        "guest":{
            "w24":{
                "enable":true,
                "ssid":"apv2_test_jokerguest-24",
                "pwd":"88888888",
                "max_num":22
            },
            "w5":{
                "enable":true,
                "ssid":"apv5_test_jokerguest-5",
                "pwd":"88888888",
                "max_num":22
            },
            "oui":"FF:FF:FF"
        }
    }' -H 'Content-Type: application/json' -k https://10.11.173.231:9900/api/cluster/devices/3e:ad:54:e1:13:4e/rpc/sysctrl/wifi/config_adv)

    echo '发送配置 2'
    echo "时间：$(date '+%Y-%m-%d %H:%M:%S')"
    echo "返回结果：$response2"
    echo '等待 100 秒'
    sleep 100
    echo '-------------------------'
done
