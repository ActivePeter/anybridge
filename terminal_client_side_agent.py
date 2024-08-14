import asyncio
from pyppeteer import launch
import time
import websockets

ID='15355213212'
PW='Peter991213'
WS=None
# BROWSER=None
PAGE=None

def handle(msg):
    # begin with helloworld
    if msg.text.startswith('helloworld'):
        print('received:', msg.text)
        if WS:
            WS.send(msg.text)


async def receive_from_websocket(websocket):
    # 从 WebSocket 服务器接收数据并打印到标准输出
    while True:
        response = await websocket.recv()
        print("recv: ", response)
        # print2(base64.b64encode(response).decode('utf-8')+"\n")
        if PAGE:
            # input to terminal
            await PAGE.keyboard.type(response)

async def start_ws():
    uri = "ws://127.0.0.1:8088"  # 替换为你的 WebSocket 服务器 URL
    async with websockets.connect(uri) as websocket:
        print("connected")
        # 启动两个任务，一个处理标准输入，一个处理 WebSocket 数据接收
        await receive_from_websocket(websocket)
        
async def webterminal():
    browser = await launch(headless=False,ignoreHTTPSErrors=True)
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


    # await asyncio.sleep(100)
    for i in range(1, 80):
        print(f'waiting for login, left {100-i} seconds')
        await asyncio.sleep(1)
    
    pages = await browser.pages()
    secondPage = pages[2]
    PAGE=secondPage

    # url=await secondPage.evaluate(() => document.location.href);
    url=secondPage.url
    print(url)
    secondPage.on('console', handle)
    
    await asyncio.sleep(60000)
        # await asyncio.sleep(1)
    await browser.close()

async def main():
    await asyncio.gather(
        start_ws(),
        webterminal()
    )
 
asyncio.run(main())