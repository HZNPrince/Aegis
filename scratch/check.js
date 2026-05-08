const https = require('https');

const req = https.request('https://api.mainnet-beta.solana.com', {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' }
}, (res) => {
    let data = '';
    res.on('data', chunk => data += chunk);
    res.on('end', () => {
        const json = JSON.parse(data);
        const sizes = {};
        for (const account of json.result) {
            const size = account.account.data[0].length;
            sizes[size] = (sizes[size] || 0) + 1;
        }
        console.log(sizes);
    });
});
req.write(JSON.stringify({
    jsonrpc: '2.0',
    id: 1,
    method: 'getProgramAccounts',
    params: ["So1endDq2YkqhipRh3WViPa8hdiSpxWy6z3Z6tMCpAo", { dataSlice: { offset: 0, length: 0 } }]
}));
req.end();
