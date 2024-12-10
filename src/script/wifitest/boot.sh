#!/bin/sh
# 使用pro基站批量升级usbdongle脚本

systemctl start sshd.socket
sleep 40

touch /run/.skip-reboot-when-offline

sleep 1
curl -X POST --data '{"status": true}' -H "Content-Type: application/json" http://127.0.0.1:5002/sysctrl/cmd/usb
sleep 1
nohup /var/home/elinker/apm auto-conf-v2 --notice-led --cmd --cmd-script /var/home/elinker/a.sh --bind-iface br0 >/dev/null 2>&1 &