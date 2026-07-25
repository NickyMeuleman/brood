To investigate:

How to send Rust thiserror annotations to javascript?
Now I duplicate these strings, once in #[error("")] on the backend and once in lib/errors.ts on the frontend

Validation without duplication:
Backend validation with the rust validator crate.
Frontend validation with the zod package.
Do I have to duplicate the logic or can I write it once and reuse and extend it?

## Database

Schema planning done via dbml format to visualize in https://dbdiagram.io/

Price data sources:
- yahoo finance

Exchange rate sources:
- Frankfurter API (filtered to only use the European Central Bank as source)

Alternative price data options:
- stooq
- stockquotes

Look up info about an instrument with:
- openFIGI
- EODHD

more data can be sourced from:
- https://marketstack.com/


figi plan:
```
curl 'https://api.openfigi.com/v3/mapping' \
    --request POST \
    --header 'Content-Type: application/json' \
    --data '[
      {"idType":"ID_ISIN","idValue":"IE00BFY0GT14","micCode":"XAMS"},
      {"idType":"ID_ISIN","idValue":"IE00BFY0GT14","micCode":"XETA"}
    ]'
```

Apparently not passing an API key in the headers limites you to 10 query objects per request.
With an API key, this increases to 100.

This returns:
```
[
    {
        "data": [
            {
                "figi": "BBG00NG1DZB5",
                "name": "SS SPDR MSCI WORLD UC-USD AC",
                "ticker": "SWRD",
                "exchCode": "NA",
                "compositeFIGI": "BBG00NG1DZ98",
                "securityType": "ETP",
                "marketSector": "Equity",
                "shareClassFIGI": "BBG00NG1CK56",
                "securityType2": "Mutual Fund",
                "securityDescription": "SWRD"
            }
        ]
    },
    {
        "data": [
            {
                "figi": "BBG00NG1CK47",
                "name": "SS SPDR MSCI WORLD UC-USD AC",
                "ticker": "SPPW",
                "exchCode": "GT",
                "compositeFIGI": "BBG00NG1CJQ6",
                "securityType": "ETP",
                "marketSector": "Equity",
                "shareClassFIGI": "BBG00NG1CK56",
                "securityType2": "Mutual Fund",
                "securityDescription": "SPPW"
            }
        ]
    }
]
```

Alternative is asking for everything and filtering by supported exchange.
Downside is it returns `exchCode`s and not MICs

Upside is you only need to know the ISIN.
```
curl 'https://api.openfigi.com/v3/mapping' \
    --request POST \
    --header 'Content-Type: application/json' \
    --data '[{"idType":"ID_ISIN","idValue":"IE00BFY0GT14"}]'
```
Same response structure, only the array within `data` will be long and the outermost array will be length 1.

