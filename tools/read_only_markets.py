"""Optional CCXT public-data adapter. No wallets, credentials, orders or transfers."""
import argparse, datetime, json, math, pathlib
EXCHANGES = {'kraken', 'coinbase', 'okx'}
def normalize_candles(rows):
    result=[];last=-1
    if len(rows)>500: raise ValueError('Maximum 500 candles')
    for row in rows:
        if len(row)!=6: raise ValueError('OHLCV requires six fields')
        timestamp,*values=row
        if isinstance(timestamp,bool) or not isinstance(timestamp,(int,float)) or not math.isfinite(timestamp) or timestamp<0 or int(timestamp)!=timestamp or timestamp<=last:
            raise ValueError('Candles require increasing integer millisecond timestamps')
        if any(isinstance(v,bool) or not isinstance(v,(int,float)) or not math.isfinite(v) or v<0 for v in values): raise ValueError('Invalid price or volume')
        opening,high,low,close,volume=values
        if high<max(opening,low,close) or low>min(opening,high,close): raise ValueError('Invalid OHLC bounds')
        result.append([int(timestamp),*values]);last=timestamp
    return result
def main():
    parser=argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--exchange',choices=sorted(EXCHANGES),required=True)
    parser.add_argument('--symbol',required=True)
    parser.add_argument('--timeframe',choices=['1m','5m','15m','1h','4h','1d'],default='1h')
    parser.add_argument('--output',type=pathlib.Path,required=True)
    args=parser.parse_args()
    # Import only after arguments are validated; the optional dependency is not required by CybOS.
    import ccxt
    exchange=getattr(ccxt,args.exchange)({'enableRateLimit':True,'timeout':10000})
    markets=exchange.load_markets()
    market=markets.get(args.symbol)
    if not market or not market.get('spot'): raise SystemExit('This spot pair is not listed by the exchange. CYBH is not launched.')
    if not exchange.has.get('fetchOHLCV') or args.timeframe not in (exchange.timeframes or {}): raise SystemExit('OHLCV/timeframe unavailable')
    candles=normalize_candles(exchange.fetch_ohlcv(args.symbol,args.timeframe,limit=200))
    payload={'schema':1,'source':'ccxt','exchange':args.exchange,'market_id':market['id'],'symbol':args.symbol,'market_type':'spot','timeframe':args.timeframe,'received_at':datetime.datetime.now(datetime.timezone.utc).isoformat(),'ohlcv':candles,'trading_enabled':False}
    # Refuse to overwrite existing exports, including symlinks.
    with args.output.open('x') as stream: json.dump(payload,stream,ensure_ascii=False,allow_nan=False,indent=2)
if __name__=='__main__':main()
