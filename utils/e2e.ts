import { ApiPromise, Keyring, WsProvider } from '@polkadot/api';
import { mnemonicGenerate } from '@polkadot/util-crypto';
import Spinnies from 'spinnies';

import { submit } from './utils';

(async () => {
  const spinnies = new Spinnies();

  const provider = new WsProvider('ws://localhost:9944');
  const keyring = new Keyring({ type: 'sr25519' });

  const chain = await ApiPromise.create({ provider });

  const alice = keyring.addFromUri('//Alice');

  const m = keyring.addFromUri(await mnemonicGenerate(12));

  spinnies.add('root', {
    text: ` User Account: ${m.address}`,
    status: 'succeed',
  });

  spinnies.add('did', {
    text: '          DID: pending...',
  });

  spinnies.add('kol', {
    text: '          KOL: pending...',
  });

  spinnies.add('nft', {
    text: '          NFT: pending...',
  });

  spinnies.add('advertiser', {
    text: '   Advertiser: pending...',
  });

  spinnies.add('ad', {
    text: 'Advertisement: pending...',
  });

  // 1. New User

  spinnies.add('preparing', {
    text: 'Registering DID...',
  });
  await submit(
    chain,
    chain.tx.balances.transfer(m.address, 3_000n * 10n ** 18n),
    alice
  );
  await submit(chain, chain.tx.did.register(null), m);
  spinnies.remove('preparing');
  const didOf = await chain.query.did.didOf(m.address);
  const did = didOf.toString();
  spinnies.succeed('did', { text: `          DID: ${did}` });

  // 2. Prepare KOL

  const k = keyring.addFromUri(await mnemonicGenerate(12));

  spinnies.add('preparing', {
    text: 'Creating KOL...',
  });
  await submit(
    chain,
    chain.tx.balances.transfer(k.address, 3_000n * 10n ** 18n),
    alice
  );
  await submit(chain, chain.tx.did.register(null), k);
  const kolOf = await chain.query.did.didOf(k.address);
  const kol = kolOf.toString();
  spinnies.update('kol', { text: `          KOL: ${kol}` });

  spinnies.update('preparing', {
    text: 'Creating NFT...',
  });
  await submit(chain, chain.tx.nft.kick(), k);
  const nftOf = await chain.query.nft.preferred(kol);
  const nft = nftOf.toString();
  spinnies.update('nft', { text: `          NFT: ${nft}` });

  spinnies.update('preparing', {
    text: 'Backing KOL...',
  });
  await submit(chain, chain.tx.nft.back(nft, 1_000n * 10n ** 18n), m);

  spinnies.update('preparing', {
    text: 'Minting...',
  });
  await submit(chain, chain.tx.nft.mint(nft, 'Test Token', 'XTT'), k);
  spinnies.remove('preparing');
  spinnies.succeed('nft');
  spinnies.succeed('kol');

  // 3. Prepare Advertiser

  const a = keyring.addFromUri(await mnemonicGenerate(12));

  spinnies.add('preparing', {
    text: 'Creating Advertiser...',
  });
  await submit(
    chain,
    chain.tx.balances.transfer(a.address, 3_000n * 10n ** 18n),
    alice
  );
  await submit(chain, chain.tx.did.register(null), a);
  const aderOf = await chain.query.did.didOf(a.address);
  const ader = aderOf.toString();
  spinnies.update('advertiser', { text: `   Advertiser: ${ader}` });

  spinnies.update('preparing', {
    text: 'Depositing to become Advertiser...',
  });
  await submit(chain, chain.tx.advertiser.deposit(1_000n * 10n ** 18n), a);
  spinnies.remove('preparing');
  spinnies.succeed('advertiser');

  // 4. Prepare Advertisement

  const tag = new Date().toISOString();

  spinnies.add('preparing', {
    text: 'Creating Tags...',
  });
  await submit(chain, chain.tx.tag.create(tag), a);

  spinnies.update('preparing', {
    text: 'Creating Advertisement...',
  });
  await submit(
    chain,
    chain.tx.ad.create(
      2n * 10n ** 18n,
      [tag],
      'ipfs://QmYwAPJzv5CZsnA625s3Xf2nemtYgPpHdWEz79ojWnPbdG',
      10,
      500000,
      1n * 10n ** 18n,
      1n * 10n ** 18n,
      50n * 10n ** 18n
    ),
    a
  );
  const adsOf = await chain.query.ad.adsOf(ader);
  const ad = (adsOf.toHuman() as any)[0];
  spinnies.update('ad', { text: `Advertisement: ${ad}` });

  spinnies.update('preparing', {
    text: 'Bidding...',
  });
  await submit(chain, chain.tx.ad.bid(ad, nft, 1n * 10n ** 18n, null, null), a);
  spinnies.succeed('ad');

  spinnies.remove('preparing');

  // 4. Payout

  spinnies.add('pay', { text: 'Paying...' });
  const before = await chain.query.assets.account(0, m.address);
  const beforeBalance = (before as any).balance.toHuman().replaceAll(',', '');
  await submit(chain, chain.tx.ad.pay(ad, nft, did, [[tag, 5]], null), a);
  const after = await chain.query.assets.account(0, m.address);
  const afterBalance = (after as any).balance.toHuman().replaceAll(',', '');
  spinnies.succeed('pay', { text: `Paid ${afterBalance - beforeBalance}` });

  await chain.disconnect();
})();
