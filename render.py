from socketserver import UnixStreamServer, StreamRequestHandler, ThreadingMixIn
import os
from wand.image import Image
from selenium import webdriver
from selenium.webdriver import FirefoxOptions
import sys
from pathlib import Path

socket = Path("/tmp/socket_zettel")

socket.touch(exist_ok=True)
socket.unlink()

width, ppp = 1200, "1.0"
# Next 2 lines are needed to specify the path to your geckodriver
geckodriver_path = "/snap/bin/geckodriver"
driver_service = webdriver.FirefoxService(executable_path=geckodriver_path)


opts = FirefoxOptions()
opts.add_argument("--headless")
opts.add_argument("--width={}".format(width))
opts.add_argument("--height=9182")

#fp = webdriver.FirefoxProfile()
#layout.css.devPixelsPerPx
opts.set_preference("layout.css.devPixelsPerPx", ppp)
#fp.DEFAULT_PREFERENCES['frozen']['layout.css.devPixelPerPx'] = 3.0
#opts.profile = fp

browser = webdriver.Firefox(service=driver_service, options=opts)
class Handler(StreamRequestHandler):
    def handle(self):
        while True:
            msg = self.rfile.readline().strip()
            if msg:
                #print("Data Recieved from client is: {}".format(msg))
                print(msg)
                obj = msg.decode("utf-8").split(",")
                zoom,name,source = int(obj[0]), obj[1], obj[2]
                print(zoom, name, source)
                browser.get("file:///{}/out.html".format(source))
                browser.execute_script("document.body.style.zoom='{}%'".format(zoom))
                browser.save_screenshot('/tmp/out2.png')

                with Image(filename="/tmp/out2.png") as img:
                    img.trim("white")
                    img.border("white", 15, 10)
                    img.save(filename=f"{source}/{name}.sixel")

                self.wfile.write(b"done")
            else:
                return

class ThreadedUnixStreamServer(ThreadingMixIn, UnixStreamServer):
    pass

with ThreadedUnixStreamServer(str(socket), Handler) as server:
    server.serve_forever()
