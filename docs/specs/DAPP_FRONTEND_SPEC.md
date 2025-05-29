# Prism Protocol dApp Frontend Specification

## 🎯 Overview

The Prism Protocol dApp provides a user-friendly web interface for claimants to discover campaigns, connect their wallets, and claim their entitled tokens. It integrates with the API server for proof retrieval and uses standard Solana wallet adapters for transaction signing.

## 🏗️ Architecture

```
┌─────────────────┐    ┌─────────────────┐    ┌─────────────────┐    ┌─────────────────┐
│   User Wallet   │───▶│  dApp Frontend  │───▶│ CLI API Server  │───▶│  Campaign DB    │
│                 │    │                 │    │                 │    │                 │
│ - Phantom       │    │ - Campaign list │    │ - Proof serving │    │ - Merkle proofs │
│ - Solflare      │    │ - Claim UI      │    │ - TX building   │    │ - Claimant data │
│ - WalletConnect │    │ - TX signing    │    │ - Validation    │    │ - Campaign meta │
└─────────────────┘    └─────────────────┘    └─────────────────┘    └─────────────────┘
```

## 🖥️ User Interface Design

### Landing Page

```tsx
// Key components and layout
interface LandingPageProps {
  campaigns: Campaign[];
  userWallet?: PublicKey;
}

const LandingPage = ({ campaigns, userWallet }: LandingPageProps) => (
  <div className="min-h-screen bg-gradient-to-br from-purple-900 to-blue-900">
    <Header />
    <Hero />
    <CampaignGrid campaigns={campaigns} userWallet={userWallet} />
    <Footer />
  </div>
);
```

### Campaign Discovery

```tsx
// Campaign card showing eligibility and claim status
interface CampaignCardProps {
  campaign: Campaign;
  eligibility?: ClaimantEligibility;
  userWallet?: PublicKey;
}

const CampaignCard = ({ campaign, eligibility, userWallet }: CampaignCardProps) => (
  <div className="bg-white/10 backdrop-blur-md rounded-xl p-6 border border-white/20">
    <div className="flex items-center justify-between mb-4">
      <h3 className="text-xl font-bold text-white">{campaign.name}</h3>
      <StatusBadge isActive={campaign.is_active} />
    </div>
    
    <div className="space-y-2 text-gray-300 mb-4">
      <p>Token: {campaign.mint_symbol}</p>
      <p>Total Claimants: {campaign.total_claimants.toLocaleString()}</p>
      <p>Cohorts: {campaign.total_cohorts}</p>
    </div>
    
    {eligibility ? (
      <EligibilityDisplay eligibility={eligibility} />
    ) : userWallet ? (
      <p className="text-gray-400">Not eligible for this campaign</p>
    ) : (
      <ConnectWalletPrompt />
    )}
  </div>
);
```

### Claim Interface

```tsx
// Detailed claim page for a specific campaign
interface ClaimPageProps {
  campaign: Campaign;
  eligibility: ClaimantEligibility;
  userWallet: PublicKey;
}

const ClaimPage = ({ campaign, eligibility, userWallet }: ClaimPageProps) => {
  const [claimStatus, setClaimStatus] = useState<ClaimStatus>('ready');
  const [selectedCohorts, setSelectedCohorts] = useState<string[]>([]);
  
  return (
    <div className="max-w-4xl mx-auto px-4 py-8">
      <CampaignHeader campaign={campaign} />
      
      <div className="grid grid-cols-1 lg:grid-cols-2 gap-8">
        <div>
          <EligibilitySummary eligibility={eligibility} />
          <CohortSelector 
            cohorts={eligibility.eligible_cohorts}
            selected={selectedCohorts}
            onChange={setSelectedCohorts}
          />
        </div>
        
        <div>
          <ClaimSummary 
            selectedCohorts={selectedCohorts}
            eligibility={eligibility}
          />
          <ClaimButton 
            disabled={claimStatus === 'claiming' || selectedCohorts.length === 0}
            onClick={() => handleClaim(selectedCohorts)}
          >
            {claimStatus === 'claiming' ? 'Claiming...' : 'Claim Tokens'}
          </ClaimButton>
        </div>
      </div>
    </div>
  );
};
```

## 🔧 Technical Implementation

### Technology Stack

```json
{
  "framework": "Next.js 14",
  "styling": "Tailwind CSS",
  "wallet": "@solana/wallet-adapter-react",
  "solana": "@solana/web3.js",
  "state": "Zustand",
  "forms": "React Hook Form",
  "ui": "Headless UI",
  "icons": "Heroicons",
  "animations": "Framer Motion"
}
```

### Project Structure

```
apps/prism-protocol-dapp/
├── src/
│   ├── components/
│   │   ├── ui/                    # Reusable UI components
│   │   ├── wallet/                # Wallet connection components
│   │   ├── campaign/              # Campaign-specific components
│   │   └── claim/                 # Claiming flow components
│   ├── pages/
│   │   ├── index.tsx             # Landing page with campaign grid
│   │   ├── campaign/
│   │   │   └── [fingerprint].tsx # Individual campaign claim page
│   │   └── api/                  # API routes (if needed)
│   ├── hooks/
│   │   ├── useCampaigns.ts       # Campaign data fetching
│   │   ├── useEligibility.ts     # Claimant eligibility checking
│   │   └── useClaimTransaction.ts # Transaction building and signing
│   ├── services/
│   │   ├── api.ts                # API server communication
│   │   ├── solana.ts             # Solana connection and utilities
│   │   └── transactions.ts       # Transaction building helpers
│   ├── types/
│   │   ├── campaign.ts           # Campaign and cohort types
│   │   ├── claim.ts              # Claiming-related types
│   │   └── api.ts                # API response types
│   └── utils/
│       ├── formatting.ts         # Number and date formatting
│       ├── validation.ts         # Form validation
│       └── constants.ts          # App constants
├── public/
│   ├── icons/                    # Token and wallet icons
│   └── images/                   # Campaign assets
├── tailwind.config.js
├── next.config.js
└── package.json
```

## 🔌 API Integration

### Campaign Discovery

```typescript
// hooks/useCampaigns.ts
interface Campaign {
  fingerprint: string;
  name: string;
  mint: string;
  mint_symbol: string;
  admin: string;
  is_active: boolean;
  total_claimants: number;
  total_cohorts: number;
  deployment?: {
    deployed_at: string;
    campaign_signature: string;
    rpc_url: string;
  };
}

export const useCampaigns = () => {
  const [campaigns, setCampaigns] = useState<Campaign[]>([]);
  const [loading, setLoading] = useState(true);
  
  useEffect(() => {
    const fetchCampaigns = async () => {
      try {
        // API server discovery endpoint (future)
        const response = await fetch('/api/campaigns');
        const data = await response.json();
        setCampaigns(data.campaigns);
      } catch (error) {
        console.error('Failed to fetch campaigns:', error);
      } finally {
        setLoading(false);
      }
    };
    
    fetchCampaigns();
  }, []);
  
  return { campaigns, loading };
};
```

### Eligibility Checking

```typescript
// hooks/useEligibility.ts
interface ClaimantEligibility {
  claimant: string;
  campaign_fingerprint: string;
  eligible_cohorts: EligibleCohort[];
  total_claimable_across_cohorts: number;
}

interface EligibleCohort {
  cohort_name: string;
  merkle_root: string;
  entitlements: number;
  assigned_vault_index: number;
  assigned_vault_pubkey: string;
  merkle_proof: string[];
  amount_per_entitlement: number;
  total_claimable: number;
}

export const useEligibility = (campaignFingerprint: string, walletPubkey?: PublicKey) => {
  const [eligibility, setEligibility] = useState<ClaimantEligibility | null>(null);
  const [loading, setLoading] = useState(false);
  
  useEffect(() => {
    if (!walletPubkey) return;
    
    const checkEligibility = async () => {
      setLoading(true);
      try {
        const response = await fetch(
          `/api/campaigns/${campaignFingerprint}/claimants/${walletPubkey.toString()}/proofs`
        );
        
        if (response.ok) {
          const data = await response.json();
          setEligibility(data);
        } else {
          setEligibility(null);
        }
      } catch (error) {
        console.error('Failed to check eligibility:', error);
        setEligibility(null);
      } finally {
        setLoading(false);
      }
    };
    
    checkEligibility();
  }, [campaignFingerprint, walletPubkey]);
  
  return { eligibility, loading };
};
```

## 💳 Wallet Integration

### Wallet Provider Setup

```tsx
// components/WalletProvider.tsx
import { WalletAdapterNetwork } from '@solana/wallet-adapter-base';
import { ConnectionProvider, WalletProvider } from '@solana/wallet-adapter-react';
import { WalletModalProvider } from '@solana/wallet-adapter-react-ui';
import {
  PhantomWalletAdapter,
  SolflareWalletAdapter,
  TorusWalletAdapter,
} from '@solana/wallet-adapter-wallets';
import { clusterApiUrl } from '@solana/web3.js';

const WalletContextProvider = ({ children }: { children: ReactNode }) => {
  const network = WalletAdapterNetwork.Mainnet; // or Devnet for testing
  const endpoint = useMemo(() => clusterApiUrl(network), [network]);
  
  const wallets = useMemo(
    () => [
      new PhantomWalletAdapter(),
      new SolflareWalletAdapter(),
      new TorusWalletAdapter(),
    ],
    []
  );
  
  return (
    <ConnectionProvider endpoint={endpoint}>
      <WalletProvider wallets={wallets} autoConnect>
        <WalletModalProvider>
          {children}
        </WalletModalProvider>
      </WalletProvider>
    </ConnectionProvider>
  );
};
```

### Claim Transaction Flow

```typescript
// hooks/useClaimTransaction.ts
export const useClaimTransaction = () => {
  const { connection } = useConnection();
  const { publicKey, sendTransaction } = useWallet();
  
  const claimTokens = async (
    campaignFingerprint: string,
    cohortName: string,
    eligibility: EligibleCohort
  ) => {
    if (!publicKey) throw new Error('Wallet not connected');
    
    try {
      // Option 1: Build transaction locally using SDK
      const claimInstruction = createClaimTokensInstruction({
        campaign: deriveCampaignAddress(campaignFingerprint),
        cohort: deriveCohortAddress(campaignFingerprint, eligibility.merkle_root),
        claimant: publicKey,
        claimReceipt: deriveClaimReceiptAddress(campaignFingerprint, publicKey),
        tokenVault: new PublicKey(eligibility.assigned_vault_pubkey),
        claimantTokenAccount: await getAssociatedTokenAddress(
          new PublicKey(campaign.mint),
          publicKey
        ),
        merkleProof: eligibility.merkle_proof.map(hex => Buffer.from(hex, 'hex')),
        entitlements: eligibility.entitlements,
        assignedVaultIndex: eligibility.assigned_vault_index,
      });
      
      const transaction = new Transaction().add(claimInstruction);
      
      // Option 2: Get pre-built transaction from API server
      // const response = await fetch(`/api/campaigns/${campaignFingerprint}/claimants/${publicKey}/build-claim-tx`, {
      //   method: 'POST',
      //   headers: { 'Content-Type': 'application/json' },
      //   body: JSON.stringify({ cohort_name: cohortName })
      // });
      // const { transaction: txBase64 } = await response.json();
      // const transaction = Transaction.from(Buffer.from(txBase64, 'base64'));
      
      // Sign and send transaction
      const signature = await sendTransaction(transaction, connection);
      
      // Wait for confirmation
      const confirmation = await connection.confirmTransaction(signature, 'confirmed');
      
      return { signature, confirmation };
    } catch (error) {
      console.error('Claim transaction failed:', error);
      throw error;
    }
  };
  
  return { claimTokens };
};
```

## 🎨 User Experience Features

### Progressive Disclosure

```tsx
// Start simple, reveal complexity as needed
const ClaimFlow = () => {
  const [step, setStep] = useState<'connect' | 'discover' | 'select' | 'confirm' | 'claim'>('connect');
  
  switch (step) {
    case 'connect':
      return <WalletConnectionStep onNext={() => setStep('discover')} />;
    case 'discover':
      return <CampaignDiscoveryStep onNext={() => setStep('select')} />;
    case 'select':
      return <CohortSelectionStep onNext={() => setStep('confirm')} />;
    case 'confirm':
      return <TransactionConfirmationStep onNext={() => setStep('claim')} />;
    case 'claim':
      return <ClaimExecutionStep />;
  }
};
```

### Real-time Updates

```typescript
// Real-time claim status using WebSocket or polling
const useClaimStatus = (signature: string) => {
  const [status, setStatus] = useState<'pending' | 'confirmed' | 'failed'>('pending');
  
  useEffect(() => {
    const checkStatus = async () => {
      try {
        const response = await connection.getSignatureStatus(signature);
        if (response.value?.confirmationStatus === 'confirmed') {
          setStatus('confirmed');
        } else if (response.value?.err) {
          setStatus('failed');
        }
      } catch (error) {
        setStatus('failed');
      }
    };
    
    const interval = setInterval(checkStatus, 1000);
    return () => clearInterval(interval);
  }, [signature]);
  
  return status;
};
```

## 📱 Mobile Responsiveness

### Mobile-First Design

```tsx
// Responsive components using Tailwind CSS
const CampaignGrid = ({ campaigns }: { campaigns: Campaign[] }) => (
  <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 xl:grid-cols-4 gap-4 md:gap-6">
    {campaigns.map((campaign) => (
      <CampaignCard key={campaign.fingerprint} campaign={campaign} />
    ))}
  </div>
);

// Mobile wallet integration
const MobileWalletButton = () => {
  const isMobile = useMediaQuery('(max-width: 768px)');
  
  return (
    <WalletMultiButton 
      className={`${isMobile ? 'w-full py-3' : 'px-4 py-2'} bg-purple-600 hover:bg-purple-700`}
    />
  );
};
```

## 🚀 Deployment Strategy

### Environment Configuration

```typescript
// config/env.ts
export const config = {
  apiBaseUrl: process.env.NEXT_PUBLIC_API_BASE_URL || 'http://localhost:3000',
  solanaRpcUrl: process.env.NEXT_PUBLIC_SOLANA_RPC_URL || 'https://api.mainnet-beta.solana.com',
  solanaNetwork: process.env.NEXT_PUBLIC_SOLANA_NETWORK || 'mainnet-beta',
  programId: process.env.NEXT_PUBLIC_PROGRAM_ID || 'PrismProgramId...',
};
```

### Docker Support

```dockerfile
# Dockerfile for dApp
FROM node:18-alpine as builder
WORKDIR /app
COPY package*.json ./
RUN npm ci
COPY . .
RUN npm run build

FROM node:18-alpine
WORKDIR /app
COPY --from=builder /app/.next ./.next
COPY --from=builder /app/node_modules ./node_modules
COPY --from=builder /app/package.json ./package.json
EXPOSE 3000
CMD ["npm", "start"]
```

### CI/CD Pipeline

```yaml
# .github/workflows/deploy-dapp.yml
name: Deploy dApp
on:
  push:
    branches: [main]
    paths: ['apps/prism-protocol-dapp/**']

jobs:
  deploy:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: actions/setup-node@v4
        with:
          node-version: '18'
      - run: npm ci
        working-directory: apps/prism-protocol-dapp
      - run: npm run build
        working-directory: apps/prism-protocol-dapp
      - name: Deploy to Vercel
        uses: amondnet/vercel-action@v25
        with:
          vercel-token: ${{ secrets.VERCEL_TOKEN }}
          vercel-org-id: ${{ secrets.VERCEL_ORG_ID }}
          vercel-project-id: ${{ secrets.VERCEL_PROJECT_ID }}
          working-directory: apps/prism-protocol-dapp
```

## 🎯 Implementation Phases

### Phase 1: Core UI (Week 4)
- [ ] Next.js project setup with Tailwind CSS
- [ ] Wallet adapter integration
- [ ] Basic campaign discovery interface
- [ ] Simple claim flow for single cohort

### Phase 2: Enhanced Features (Week 5)
- [ ] Multi-cohort claiming interface
- [ ] Transaction status tracking
- [ ] Error handling and user feedback
- [ ] Mobile responsive design

### Phase 3: Production Polish (Week 6)
- [ ] Performance optimization
- [ ] SEO and accessibility
- [ ] Analytics integration
- [ ] Deployment automation

### Phase 4: Advanced Features (Future)
- [ ] Campaign search and filtering
- [ ] Claim history and receipts
- [ ] Social sharing of claims
- [ ] Campaign creator dashboard 