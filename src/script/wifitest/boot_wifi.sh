#!/bin/sh
# 更新后的wifi产测boot文件

systemctl start sshd.socket
sleep 20
touch /run/.skip-reboot-when-offline

sleep 120
curl -X POST --data '{ "netmask": "255.255.255.0", "start": "172.16.250.100", "end": "172.16.250.200" }' -H "Content-Type: application/json" http://127.0.0.1:5002/sysctrl/conf/internal_dhcp
sleep 50
curl -v  -X POST --data '{}' -H "Content-Type: application/json" http://127.0.0.1:5003/sysctrl/wifi/product_conf_test
