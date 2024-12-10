
#!/bin/bash

# 产看
curl -X POST --data '{"sid":"", "name":"fastfs"}' -H "Content-Type: application/json" \
    http://127.0.0.1:5002/sysctrl/container/start

sleep 2
# 查看fastfs状态
curl -k -d '{"remote_addr":"10.11.107.88", "remote_port":37028}' \
    -H 'Content-Type: application/json' \
    https://127.0.0.1:5004/sysctrl/fastfs/sock_start
	
sleep 2
# 查看fastfs状态
curl -k https://127.0.0.1:5004/sysctrl/fastfs/sock_status

sleep 2

iptables -t nat -nvL PREROUTING