@echo off
:: 访问blob并获取JSON响应
echo Fetching data from happ blob

:: 第一步：发送初始请求,获取日志列表
curl -s -X POST -d "{\"file\":[],\"sid\":\"a\"}" -H "Content-Type: application/json" -k https://172.16.120.29:9900/api/cluster/devices/98:6d:35:72:24:7a/rpc/sysctrl/log/upload 
timeout /t 3 /nobreak >nul

echo 
echo Please input logname
set /p file=

:: 检查输入是否为空
if "%file%"=="" (
    echo No filename provided. Exiting.
    pause
    exit /b
)

:: 第三步：下载指定的文件
echo Downloading file: %file% ...

curl -s -X POST -d "{\"file\":[%file%],\"sid\":\"a\"}" -H "Content-Type: application/json" -k https://172.16.120.29:9900/api/cluster/devices/98:6d:35:72:24:7a/rpc/sysctrl/log/upload

echo upload success
echo 

echo  mac地址大写
curl -k  https://172.16.120.29:9900/api/blob/siyingyu/0006/aplog/986D3572247A/messages --output a.gz