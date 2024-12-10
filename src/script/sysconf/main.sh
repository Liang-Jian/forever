#!/bin/bash
# 执行脚本文件，并返回结果。从外面调用，测试API

################################
# sysctrl/ping
# sysctrl/conf/vendor_get
# sysctrl/conf/desc_get
# sysctrl/conf/desc
# sysctrl/conf/server_get
# sysctrl/conf/net_get
# sysctrl/conf/web_get
# sysctrl/conf/wifi_get
# sysctrl/conf/timezone_get
# sysctrl/conf/internal_dhcp_get
# sysctrl/conf/dhcp_options_get
#################################
cmd_file="cmd.txt"

# 检查文件是否存在
if [ ! -f "$cmd_file" ]; then
    echo "File $cmd_file not found!"
    exit 1
fi

# 将多行命令合并为一行
current_command=""
while IFS= read -r line || [[ -n "$line" ]]; do
    # 如果行不为空，拼接到当前命令
    if [[ -n "$line" ]]; then
        current_command+="$line"
    else
        # 如果遇到空行，执行当前命令并清空
        eval "$current_command"
        sleep 10
        current_command=""
    fi
done < "$cmd_file"

# 处理最后一条命令
if [[ -n "$current_command" ]]; then
    eval "$current_command"
    sleep 10
fi
