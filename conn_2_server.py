import asyncio
from pyppeteer import launch
import time

ID='15355213212'
PW='Peter991213'


def handle(msg):
    # begin with helloworld
    if msg.text.startswith('helloworld'):
        print('received:', msg.text)

async def main():
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

    # url=await secondPage.evaluate(() => document.location.href);
    url=secondPage.url
    print(url)
    secondPage.on('console', handle)
    
    await asyncio.sleep(60000)
        # await asyncio.sleep(1)
    await browser.close()
 
asyncio.get_event_loop().run_until_complete(main())