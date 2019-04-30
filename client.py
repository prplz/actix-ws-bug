import asyncio

import aiohttp


async def main():
    url = "http://127.0.0.1:8080/ws/"
    for _ in range(10):
        await aiohttp.ClientSession().ws_connect(url, autoclose=False, autoping=False)


if __name__ == "__main__":
    loop = asyncio.get_event_loop()
    loop.run_until_complete(main())
