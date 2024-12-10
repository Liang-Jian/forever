
#!/bin/sh

# curl -X  POST -d '{"file":[],"sid":"dick"}' -H 'Content-Type: application/json' -k https://10.11.173.231:9900/api/cluster/devices/3e:ad:54:e1:13:4e/rpc/sysctrl/log/upload
# curl -X  POST -d '{"file":["messages"],"sid":"dick"}' -H 'Content-Type: application/json' -k https://10.11.173.231:9900/api/cluster/devices/3e:ad:54:e1:13:4e/rpc/sysctrl/log/upload

# curl -X  POST -d '{}' -H 'Content-Type: application/json' -k https://10.11.173.231:9900/api/cluster/devices/3e:ad:54:e1:13:4e/rpc/sysctrl/cmd/ability_get


# service conf get
# curl -X  POST -d '{}' -H 'Content-Type: application/json' -k https://10.11.173.231:9900/api/cluster/devices/3e:ad:54:e1:13:4e/rpc/sysctrl/conf/server_get

# network conf get 
# curl -X  POST -d '{}' -H 'Content-Type: application/json' -k https://10.11.173.231:9900/api/cluster/devices/3e:ad:54:e1:13:4e/rpc/sysctrl/conf/net_get

# usb_get
# curl -X  POST -d '{}' -H 'Content-Type: application/json' -k https://10.11.173.231:9900/api/cluster/devices/3e:ad:54:e1:13:4e/rpc/sysctrl/cmd/usb_get

# curl -X  POST -d '{}' -H 'Content-Type: application/json' -k  https://10.11.173.231:9900/api/cluster/devices/3e:ad:54:e1:13:4e/rpc/sysctrl/wifi/config_get


# curl -X  POST -d '{}' -H 'Content-Type: application/json' -k https://10.11.173.231:9900/api/cluster/devices/3e:ad:54:e1:13:4e/rpc/sysctrl/container/status

curl -X  POST -d '{}' -H 'Content-Type: application/json' -k  https://10.11.173.231:9900/api/cluster/devices/3e:ad:54:e1:13:4e/rpc/sysctrl/conf/vendor_get