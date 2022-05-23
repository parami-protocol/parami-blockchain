import { ApiPromise } from '@polkadot/api';
import { SubmittableExtrinsic } from '@polkadot/api/types';
import { KeyringPair } from '@polkadot/keyring/types';
import { DispatchError } from '@polkadot/types/interfaces';

export const parseError = (chain: ApiPromise, error: DispatchError) => {
  const decoded = chain.registry.findMetaError(error.asModule);
  const { docs, name, section } = decoded;

  return `${section}.${name}: ${docs.join(' ')}`;
};

export const submit = (
  chain: ApiPromise,
  extrinsic: SubmittableExtrinsic<'promise'>,
  keypair: KeyringPair
): Promise<{ tx: string; block: string }> => {
  return new Promise((resolve, reject) => {
    try {
      extrinsic.signAndSend(
        keypair,
        { nonce: -1 },
        ({ events, status, dispatchError }) => {
          if (dispatchError) {
            reject(new Error(parseError(chain, dispatchError)));
          } else if (status.isInBlock) {
            resolve({
              tx: extrinsic.hash.toHex(),
              block: status.asInBlock.toHex(),
            });
          }
        }
      );
    } catch (e) {
      reject(e);
    }
  });
};
