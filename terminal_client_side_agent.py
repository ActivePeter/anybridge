import asyncio
from pyppeteer import launch
import time
import websockets
import base64

ID='15355213212'
PW='Peter991213'

class Global:
    WS=None
    # BROWSER=None
    PAGE=None
    HANDLE_BUFF=""
GLOBAL=Global()


def handle(msg):
    # begin with helloworld
    if msg.text.startswith('helloworld'):
        if '"data":"' not in msg.text:
            print("incoming not match 1")
            return
        if GLOBAL.HANDLE_BUFF.find("rmrp:")==-1:
            # join the first msg with rmrp
            if msg.text.find("rmrp:")==-1:
                print("incoming not match 2")
                return

        msg=GLOBAL.HANDLE_BUFF+msg.text.split('"data":"')[1].split('"')[0]
        msgparts=msg.split("\\r\\n")

        # for each except the last
        for i in range(len(msgparts)-1):
            msg=msgparts[i]
            print("one remote msg pack",msg)
            if GLOBAL.WS:
                print("send to ws")
                # msg decode base64 to bytes
                msg=base64.b64decode(msg.split("rmrp:")[1])
                # create async call in sync function
                asyncio.create_task(GLOBAL.WS.send(msg))
        
        # append_last
        if msgparts[-1].find("rmrp")!=-1:
            GLOBAL.HANDLE_BUFF+=msgparts[-1]
        print("left unhandled",GLOBAL.HANDLE_BUFF)
        # print('received:', msg.text)
        # if msg.text.find("rmrp:")!=-1:
        #     rpmps=msg=msg.text.split("rmrp:")
        #     for i in range(len(rpmps)-1):
        #         msg=rpmps[i+1]
        #         msg=msg.split("\\r\\n")[0]
        #         if msg.find("\"")!=-1:
        #             msg=msg.split("\"")[0]
        #         print("one remote msg pack",msg)
        #         if GLOBAL.WS:
        #             print("send to ws")
        #             # msg decode base64 to bytes
        #             msg=base64.b64decode(msg)
        #             # create async call in sync function
        #             asyncio.create_task(GLOBAL.WS.send(msg))
                    # GLOBAL.WS.send(msg)

async def input(content):
    for c in content:
        # await GLOBAL.PAGE.keyboard.type(c)
        await GLOBAL.PAGE.keyboard.press(c)
async def receive_from_websocket(websocket):
    # 从 WebSocket 服务器接收数据并打印到标准输出
    while True:
        response = await websocket.recv()
        print("\n>>>")
        print("recv from proxy: ", response)
        # print2(base64.b64encode(response).decode('utf-8')+"\n")
        if GLOBAL.PAGE:
            # input to terminal
            content=base64.b64encode(response).decode('utf-8')
            print("forward to web terminal as base64",content)
            
            await input(content)
            # await GLOBAL.PAGE.keyboard.type(content)
            await asyncio.sleep(0.1)
            # enter
            await GLOBAL.PAGE.keyboard.press('Enter')
            await asyncio.sleep(0.05)
        else:
            print("no page to forward")

async def start_ws():
    uri = "ws://127.0.0.1:8089"  # 替换为你的 WebSocket 服务器 URL
    async with websockets.connect(uri) as websocket:
        print("connected")
        GLOBAL.WS=websocket
        # 启动两个任务，一个处理标准输入，一个处理 WebSocket 数据接收
        await receive_from_websocket(websocket)
        
async def webterminal():
    browser = await launch(headless=False,ignoreHTTPSErrors=True)
    # allow https no check disable tls in flags

    page = await browser.newPage()
    await page.goto('https://oauth.dianxin.com/#/main/welcome')
    
    await asyncio.sleep(10)
    # find class inputLogin
    inputLogin=await page.querySelector('.inputLogin')
    # find class posRel
    posRel=await page.querySelector('.posRel')

    # input ID
    await inputLogin.type(ID)
    # click next
    await posRel.click()
    await asyncio.sleep(1)
    # input PW
    await posRel.type(PW)
    # click login
    await page.click('.loginBt')


    await asyncio.sleep(4)
    while True:
        print(f'waiting for webterminal')
        await asyncio.sleep(10)
        try:
            pages = await browser.pages()
        except:
            continue

        if len(pages)<3:
            continue

        secondPage = pages[2]
        GLOBAL.PAGE=secondPage
        # url=await secondPage.evaluate(() => document.location.href);
        url=secondPage.url
        if url.find("/ssh/")!=-1:
            break
    
    # pages = await browser.pages()
    # secondPage = pages[2]
    # GLOBAL.PAGE=secondPage

    # url=await secondPage.evaluate(() => document.location.href);
    url=GLOBAL.PAGE.url
    print(url)
    GLOBAL.PAGE.on('console', handle)

    await input("ssh teleinfra@10.127.20.218")
    await GLOBAL.PAGE.keyboard.press('Enter')
    await asyncio.sleep(2)
    await input("Jxbnhvp4$")
    await GLOBAL.PAGE.keyboard.press('Enter')

    print("\n waiting for end")
    await asyncio.sleep(60000)
        # await asyncio.sleep(1)
    await browser.close()

async def main():
    await asyncio.gather(
        start_ws(),
        webterminal()
    )
 
asyncio.run(main())