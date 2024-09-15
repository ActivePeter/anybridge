import base64, time, sys
import threading,time
import urllib.request
import websockets, asyncio, gc

with open("/Users/hello/logs/crtplugin.log", "w") as f:
    f.write("")

class Global:
    QUEUE_LOCK=threading.Lock()
    QUEUE=b""
    OBJ_TAB=None
    RUNNING=True
    SEND_DATA_LOCK=threading.Lock()
    SEND_DATA=b""
    LOGFILE=open("/Users/hello/logs/crtplugin.log", "a")
    WS=None
    EVENTLOOP=None
# queue=[]
GLOBAL=Global()


# redirect stdio and stderr to file
sys.stdout = GLOBAL.LOGFILE
# pipe stderr to stdout
sys.stderr = sys.stdout


def debug(v):
    sys.stdout.write(v+"\n")
    sys.stdout.flush()

PORT=8091

# async def send_loop():
#     while True:
#         data=b''
#         with GLOBAL.SEND_DATA_LOCK:
#             data=GLOBAL.SEND_DATA
#             GLOBAL.SEND_DATA=b""
#         if len(data)>0:
#             debug(f"forward remote msg to tcp proxy {data}")
#             await GLOBAL.WS.send(data)
#         await asyncio.sleep(0.05)
async def receive_loop():
    while True:
        debug("\n>>> waiting for proxied data...")
        response = await GLOBAL.WS.recv()
        debug(f"recv from proxied tcp: {response}")
        with GLOBAL.QUEUE_LOCK:
        	GLOBAL.QUEUE+=response
            # GLOBAL.QUEUE.append(response)
            
async def start_ws():
    GLOBAL.EVENTLOOP = asyncio.get_running_loop()
    uri = f"ws://127.0.0.1:{PORT}"  # 替换为你的 WebSocket 服务器 URL
    async with websockets.connect(uri) as websocket:
        # builtins.print=log
        debug("connected")
        GLOBAL.WS=websocket
        await asyncio.gather(
            # send_loop(),
            receive_loop()
        )



    # GLOBAL.LOGFILE.write(v+"\n")
    # GLOBAL.LOGFILE.flush()

# def test_queue():
#     # GLOBAL.QUEUE.append("test_queue\n")
#     while True:
#         data=""
#         with GLOBAL.SEND_DATA_LOCK:
#             data=GLOBAL.SEND_DATA
#             GLOBAL.SEND_DATA=""
		
#         try:
#             resp=urllib.request.urlopen(f"http://127.0.0.1:{PORT}/{data}",timeout=3)
#         except:
#             break
#         resp=resp.read()
#         if len(resp)>0:
#             debug("pulled target data, forward to remote")
#             debug(f"  pulled as follow {resp}")
#             with GLOBAL.QUEUE_LOCK:
#                 # GLOBAL.QUEUE.append(resp)
#                 GLOBAL.QUEUE+=resp
#         else:
#             debug("nothing to forward")
#         # time.sleep(0.2)
#     GLOBAL.RUNNING=False
#     debug("end running")
		


def print2(v):
    # sys.stdout.write(v); sys.stdout.flush()
    # print(v)
        
	# encode v with base64
    if isinstance(v,str): v=v.encode()
    v=base64.b64encode(v).decode()
    debug(f"forward tcp to remote, base64: {v}")
    chunk_size=1024
    vs=[v[i:i + chunk_size] for i in range(0, len(v), chunk_size)]
    for i, v in enumerate(vs):
        if i==len(vs)-1:
            GLOBAL.OBJ_TAB.Screen.Send(v+"*\n")
        else:
            GLOBAL.OBJ_TAB.Screen.Send(v+"\n")
		

# def debug(v):
# 	# GLOBAL.OBJ_TAB.Screen.Send(v)
#     GLOBAL.DEBUG_QUEUE.append(v)

    # debug("begin receive")
    # while GLOBAL.RUNNING:
    #     GLOBAL.OBJ_TAB.Screen.WaitForString('rmrp:', 1)
    #     GLOBAL.SEND_DATA += GLOBAL.OBJ_TAB.Screen.ReadString(":rmrpe")
    #     debug("received "+GLOBAL.SEND_DATA)
        # time.sleep(1)


# t.join()
def Main():
    GLOBAL.OBJ_TAB=crt.GetScriptTab()
    debug("script bgin")

    def ws_therad():
        asyncio.run(start_ws())
    t=threading.Thread(target=ws_therad)
    t.start()
    time.sleep(5)
    
    # t = threading.Thread(target=test_queue)
    # t.start()
    
    cnt=0
    # objTab = crt.GetScriptTab()
    while GLOBAL.RUNNING:
        gc.collect()
        # debug(f"loop cnt: {cnt}")
        take=b""
        with GLOBAL.QUEUE_LOCK:
            take=GLOBAL.QUEUE
            GLOBAL.QUEUE=b""
        
        if len(take) >0:
            # debug(f"forward to remote queue len: {len(GLOBAL.QUEUE)}")
            print2(take)
        else:
            GLOBAL.OBJ_TAB.Screen.Send("*\n")

        # GLOBAL.OBJ_TAB.Screen.WaitForString('rmrp:')
        # debug("received rmrp")
        # debug(f"prev received {GLOBAL.SEND_DATA}")
        
		## http mode senddata
		# with GLOBAL.SEND_DATA_LOCK:
        #     GLOBAL.SEND_DATA += GLOBAL.OBJ_TAB.Screen.ReadString(":rmrpe").split("rmrp:")[-1]
        
		## websocket mode senddata
        base64str=GLOBAL.OBJ_TAB.Screen.ReadString(":rmrpe").split("rmrp:")[-1]
        if len(base64str) > 0:
            debug(f"forward remote to tcp proxy, base64: {base64str}")
            data=base64.b64decode(base64str)
            chunk_size=4096
            ds=[data[i:i + chunk_size] for i in range(0, len(data), chunk_size)]
            # with GLOBAL.SEND_DATA_LOCK:
            #     GLOBAL.SEND_DATA += decoded
            for d in ds:
                asyncio.run_coroutine_threadsafe(GLOBAL.WS.send(d),loop = GLOBAL.EVENTLOOP)

        
        
		
		# debug(f"received {GLOBAL.SEND_DATA}")
        # GLOBAL.OBJ_TAB.Screen.WaitForString('\r', 1)
        
        
        time.sleep(0.2)
        cnt+=1
Main()