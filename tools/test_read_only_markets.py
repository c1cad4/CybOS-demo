import unittest
from read_only_markets import normalize_candles
class MarketData(unittest.TestCase):
    def test_valid_public_candles(self):self.assertEqual(normalize_candles([[1000,1,2,0.5,1.5,20]])[0][0],1000)
    def test_invalid_and_out_of_order_candles_rejected(self):
        for rows in [[[1000,1,float('nan'),0,1,2]],[[1000,1,0.5,0,1,2]],[[1000,1,2,0,1,2],[999,1,2,0,1,2]],[[True,1,2,0,1,2]],[[1000,1,2,0,1,2]]*501]:
            with self.assertRaises(ValueError):normalize_candles(rows)
if __name__=='__main__':unittest.main()
