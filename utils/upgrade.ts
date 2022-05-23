import fs from 'fs';

import { ApiPromise, Keyring, WsProvider } from '@polkadot/api';
import Spinnies from 'spinnies';

(async () => {
  const spinnies = new Spinnies();

  const provider = new WsProvider(process.env.DEPLOY_TARGET_RPC);
  const keyring = new Keyring({ type: 'sr25519' });

  const chain = await ApiPromise.create({ provider });

  const keypair = keyring.addFromMnemonic(
    process.env.DEPLOY_MNEMONIC || '//Alice'
  );

  spinnies.add('code', {
    text: 'Loading Code...',
  });
  const code = fs
    .readFileSync(
      process.env.DEPLOY_CODE_FILE ||
        'parami_dana_runtime.compact.compressed.wasm'
    )
    .toString('hex');
  spinnies.succeed('code', { text: ` ${code.length / 2} bytes loaded` });

  spinnies.add('sudo', {
    text: 'Setting Code...',
  });
  chain.tx.sudo
    .sudoUncheckedWeight(chain.tx.system.setCode(`0x${code}`))
    .signAndSend(keypair, ({ events = [], status }) => {
      if (status.isFinalized) {
        spinnies.succeed('sudo', { text: ` ${status.asFinalized.toHex()}` });
      } else {
        spinnies.update('sudo', { text: ` ${status.type}` });
      }
    });
})();
