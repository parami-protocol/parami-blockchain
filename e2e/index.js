import { ApiPromise, Keyring, WsProvider } from '@polkadot/api';
import { mnemonicGenerate } from '@polkadot/util-crypto';
import Spinnies from 'spinnies';

import { submit } from './utils.js';

(async () => {
  const spinnies = new Spinnies();

  const provider = new WsProvider('ws://localhost:9944');
  const keyring = new Keyring({ type: 'sr25519' });

  const chain = await ApiPromise.create({ provider });

  const alice = keyring.addFromUri('//Alice');

  const m = keyring.addFromUri(await mnemonicGenerate(12));
  const c = keyring.addFromUri(await mnemonicGenerate(12));

  spinnies.add('root', {
    text: ` Root Account: ${m.address}`,
    status: 'succeed',
  });

  spinnies.add('codo', {
    text: ` Codo Account: ${c.address}`,
    status: 'succeed',
  });

  spinnies.add('cash', {
    text: ' Cash Account: pending...',
  });

  spinnies.add('did', {
    text: '          DID: pending...',
  });

  spinnies.add('kol', {
    text: '          KOL: pending...',
  });

  spinnies.add('advertiser', {
    text: '   Advertiser: pending...',
  });

  spinnies.add('ad', {
    text: 'Advertisement: pending...',
  });

  // 1. New User

  spinnies.add('preparing', {
    text: 'Airdropping...',
  });
  await submit(
    chain,
    chain.tx.balances.transfer(c.address, 3_000n * 10n ** 18n),
    alice
  );
  spinnies.remove('preparing');

  await submit(
    chain,
    chain.tx.magic.createStableAccount(m.address, 2_000n * 10n ** 18n),
    c
  );
  const stableOf = await chain.query.magic.metadata(c.address);
  const s = stableOf.toHuman().stashAccount;
  spinnies.succeed('cash', { text: ` Cash Account: ${s}` });

  spinnies.add('preparing', {
    text: 'Registering DID...',
  });
  await submit(chain, chain.tx.magic.codo(chain.tx.did.register(null)), c);
  spinnies.remove('preparing');

  const didOf = await chain.query.did.didOf(s);
  const did = didOf.toString();
  spinnies.succeed('did', { text: `          DID: ${did}` });

  // 2. Prepare KOL

  const km = keyring.addFromUri(await mnemonicGenerate(12));
  const kc = keyring.addFromUri(await mnemonicGenerate(12));

  spinnies.add('preparing', {
    text: 'Creating KOL...',
  });

  await submit(
    chain,
    chain.tx.balances.transfer(kc.address, 3_000n * 10n ** 18n),
    alice
  );

  await submit(
    chain,
    chain.tx.magic.createAccountsAndDid(km.address, 2_000n * 10n ** 18n, null),
    kc
  );

  const ks = await chain.query.magic.metadata(kc.address);
  const kolOf = await chain.query.did.didOf(ks.toHuman().stashAccount);
  const kol = kolOf.toString();
  spinnies.update('kol', { text: `          KOL: ${kol}` });

  spinnies.update('preparing', {
    text: 'Backing KOL...',
  });
  await submit(
    chain,
    chain.tx.magic.codo(chain.tx.nft.back(kol, 1_000n * 10n ** 18n)),
    c
  );

  spinnies.update('preparing', {
    text: 'Minting...',
  });
  await submit(
    chain,
    chain.tx.magic.codo(chain.tx.nft.mint('Test Token', 'XTT')),
    kc
  );
  spinnies.remove('preparing');
  spinnies.succeed('kol');

  // 3. Prepare Advertiser

  const am = keyring.addFromUri(await mnemonicGenerate(12));
  const ac = keyring.addFromUri(await mnemonicGenerate(12));

  spinnies.add('preparing', {
    text: 'Creating Advertiser...',
  });

  await submit(
    chain,
    chain.tx.balances.transfer(ac.address, 3_000n * 10n ** 18n),
    alice
  );

  await submit(
    chain,
    chain.tx.magic.createAccountsAndDid(am.address, 2_000n * 10n ** 18n, null),
    ac
  );

  const as = await chain.query.magic.metadata(ac.address);
  const aderOf = await chain.query.did.didOf(as.toHuman().stashAccount);
  const ader = aderOf.toString();
  spinnies.update('advertiser', { text: `   Advertiser: ${ader}` });

  spinnies.update('preparing', {
    text: 'Depositing to become Advertiser...',
  });
  await submit(
    chain,
    chain.tx.magic.codo(chain.tx.advertiser.deposit(1_000n * 10n ** 18n)),
    ac
  );
  spinnies.remove('preparing');
  spinnies.succeed('advertiser');

  // 4. Prepare Advertisement

  const tag = new Date().toISOString();

  spinnies.add('preparing', {
    text: 'Creating Tags...',
  });
  await submit(chain, chain.tx.magic.codo(chain.tx.tag.create(tag)), ac);

  spinnies.update('preparing', {
    text: 'Creating Advertisement...',
  });
  await submit(
    chain,
    chain.tx.magic.codo(
      chain.tx.ad.create(
        2n * 10n ** 18n,
        [tag],
        'ipfs://QmYwAPJzv5CZsnA625s3Xf2nemtYgPpHdWEz79ojWnPbdG',
        10,
        500000
      )
    ),
    ac
  );
  const adsOf = await chain.query.ad.adsOf.entries();
  const ad = adsOf[0][1].toHuman()[0];
  spinnies.update('ad', { text: `Advertisement: ${ad}` });

  spinnies.update('preparing', {
    text: 'Bidding...',
  });
  await submit(
    chain,
    chain.tx.magic.codo(chain.tx.ad.bid(ad, kol, 1n * 10n ** 18n)),
    ac
  );
  spinnies.succeed('ad');

  spinnies.remove('preparing');

  // 4. Payout

  spinnies.add('pay', { text: 'Paying...' });
  const before = await chain.query.assets.account(0, s);
  const beforeBalance = before.balance.toHuman().replaceAll(',', '');
  await submit(
    chain,
    chain.tx.magic.codo(chain.tx.ad.pay(ad, kol, did, [[tag, 5]], null)),
    ac
  );
  const after = await chain.query.assets.account(0, s);
  const afterBalance = after.balance.toHuman().replaceAll(',', '');
  spinnies.succeed('pay', { text: `Paid ${afterBalance - beforeBalance}` });

  await chain.disconnect();
})();
