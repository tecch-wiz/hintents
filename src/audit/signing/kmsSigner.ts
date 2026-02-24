import { KMSClient, SignCommand } from '@aws-sdk/client-kms';
import type { AuditSigner, PublicKey, Signature } from './types';

export class KmsEd25519Signer implements AuditSigner {
  private readonly kmsClient: KMSClient;
  private readonly keyId: string;
  private readonly publicKeyPem: string;

  constructor() {
    const keyId = process.env.ERST_KMS_KEY_ID;
    const publicKeyPem = process.env.ERST_KMS_PUBLIC_KEY_PEM;
    const region = process.env.ERST_KMS_REGION || 'us-east-1';

    if (!keyId) {
      throw new Error('KMS signer selected but ERST_KMS_KEY_ID is not set');
    }
    if (!publicKeyPem) {
      throw new Error('KMS signer selected but ERST_KMS_PUBLIC_KEY_PEM is not set');
    }

    this.keyId = keyId;
    this.publicKeyPem = publicKeyPem;
    this.kmsClient = new KMSClient({ region });
  }

  async sign(payload: Uint8Array): Promise<Signature> {
    try {
      const cmd = new SignCommand({
        KeyId: this.keyId,
        Message: Buffer.from(payload),
        SigningAlgorithm: 'Ed25519',
        MessageFormat: 'RAW',
      });

      const response = await this.kmsClient.send(cmd);

      if (!response.Signature) {
        throw new Error('KMS Sign response missing signature');
      }

      return Buffer.from(response.Signature);
    } catch (e) {
      const msg = e instanceof Error ? e.message : String(e);
      throw new Error(`kms signing failed: ${msg}`);
    }
  }

  async public_key(): Promise<PublicKey> {
    return this.publicKeyPem;
  }
}
