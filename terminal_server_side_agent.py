import asyncio
import websockets
import sys
import builtins
import logging
import base64


# print("start subprocess agent")

# 配置 logging 模块，将日志写入文件
logging.basicConfig(filename='app.log',  # 日志文件名
                    level=logging.DEBUG,  # 设置日志级别
                    format='%(asctime)s - %(levelname)s - %(message)s')  # 日志格式


old_stdout=sys.stdout

def log(*args, **kwargs):
    sys.stdout.write(' '.join(map(str,args))+'\n')
    sys.stdout.flush()

def print2(*args, **kwargs):
    print("print2", args, kwargs)
    old_stdout.write(' '.join(map(str,args)))
    old_stdout.flush()

# 重定向 stdout 和 stderr 到文件
def redirect_output_to_file(output_file):
    sys.stdout = open(output_file, 'w')
    sys.stderr = sys.stdout
redirect_output_to_file('agent.log')


# exit(0)

async def read_stdin(websocket):
    # 从标准输入读取数据并发送到 WebSocket 服务器
    while True:
        line = await asyncio.get_event_loop().run_in_executor(None, sys.stdin.readline)
        if line:
            assert line[-1] == '\n'
            line=line[:-1]
            decode=base64.b64decode(line)
            print("send: ", decode)
            await websocket.send(decode)
        else:
            break

async def receive_from_websocket(websocket):
    # 从 WebSocket 服务器接收数据并打印到标准输出
    while True:
        response = await websocket.recv()
        print("recv: ", response)
        print2(base64.b64encode(response).decode('utf-8')+"\n")

async def start_ws():
    uri = "ws://127.0.0.1:8087"  # 替换为你的 WebSocket 服务器 URL
    async with websockets.connect(uri) as websocket:
        builtins.print=log
        print("connected")
        # 启动两个任务，一个处理标准输入，一个处理 WebSocket 数据接收
        await asyncio.gather(
            read_stdin(websocket),
            receive_from_websocket(websocket)
        )

async def main():
    await start_ws()

# 运行异步事件循环
asyncio.run(main())
