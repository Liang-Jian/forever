
#!/bin/bash



curl -s -X POST --data '{}'  -k https://10.11.23.250:9900/api/cluster/devices/5E:4F:81:0A:5A:E5/rpc/sysctrl/ping
   

#!/bin/sh
"MAC"="$1"
if [ "$MAC" = ""]; then
    echo $0 Mac
    exit
fi
