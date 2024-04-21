use super::*;

#[derive(Debug, Parser)]
pub(crate) struct Mint {
  #[clap(long, help = "Use <FEE_RATE> sats/vbyte for mint transaction.")]
  fee_rate: FeeRate,
  #[clap(long, help = "Mint <RUNE>. May contain `.` or `•`as spacers.")]
  rune: SpacedRune,
  #[clap(
    long,
    help = "Include <AMOUNT> postage with mint output. [default: 10000sat]"
  )]
  postage: Option<Amount>,
  #[clap(long, help = "Send minted runes to <DESTINATION>.")]
  destination: Option<Address<NetworkUnchecked>>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Output {
  pub rune: SpacedRune,
  pub pile: Pile,
  pub mint: Txid,
}

impl Mint {
  pub(crate) fn run(self, wallet: Wallet) -> SubcommandResult {
    ensure!(
      wallet.has_rune_index(),
      "`ord wallet mint` requires index created with `--index-runes` flag",
    );

    let rune = self.rune.rune;

    let bitcoin_client = wallet.bitcoin_client();

    let block_height = bitcoin_client.get_block_count()?;

    let Some((id, rune_entry, _)) = wallet.get_rune(rune)? else {
      bail!("rune {rune} has not been etched");
    };

    let postage = self.postage.unwrap_or(TARGET_POSTAGE);

    let amount = rune_entry
      .mintable(block_height)
      .map_err(|err| anyhow!("rune {rune} {err}"))?;

    let chain = wallet.chain();

    let destination = match self.destination {
      Some(destination) => destination.require_network(chain.network())?,
      None => wallet.get_change_address()?,
    };

    ensure!(
      destination.script_pubkey().dust_value() < postage,
      "postage below dust limit of {}sat",
      destination.script_pubkey().dust_value().to_sat()
    );

    let runestone = Runestone {
      mint: Some(id),
      ..default()
    };

    let script_pubkey = runestone.encipher();

    ensure!(
      script_pubkey.len() <= 82,
      "runestone greater than maximum OP_RETURN size: {} > 82",
      script_pubkey.len()
    );

    let unfunded_transaction = Transaction {
      version: 2,
      lock_time: LockTime::ZERO,
      input: Vec::new(),
      output: vec![
        TxOut {
          script_pubkey,
          value: 0,
        },
        TxOut {
          script_pubkey: destination.script_pubkey(),
          value: postage.to_sat(),
        },
      ],
    };

    wallet.lock_non_cardinal_outputs()?;

    let unsigned_transaction =
      fund_raw_transaction(bitcoin_client, self.fee_rate, &unfunded_transaction)?;

    let signed_transaction = bitcoin_client
      .sign_raw_transaction_with_wallet(&unsigned_transaction, None, None)?
      .hex;

    let signed_transaction = consensus::encode::deserialize(&signed_transaction)?;

    assert_eq!(
      Runestone::decipher(&signed_transaction),
      Some(Artifact::Runestone(runestone)),
    );

    let transaction = bitcoin_client.send_raw_transaction(&signed_transaction)?;

    Ok(Some(Box::new(Output {
      rune: self.rune,
      pile: Pile {
        amount,
        divisibility: rune_entry.divisibility,
        symbol: rune_entry.symbol,
      },
      mint: transaction,
    })))
  }
}

#[derive(Debug)]
pub struct RunesMint {
  pub fee_rate: FeeRate,
  pub rune: SpacedRune,
  pub postage: Option<Amount>,
  pub destination: Option<Address<NetworkUnchecked>>,
}

impl RunesMint {
  pub fn run_in_place(self, params: WalletParams) -> anyhow::Result<Vec<u8>> {
    // 打印构造钱包的参数
    log::debug!("Constructed wallet with params: {:?}", params);

    let wallet = params.constructor()?;

    self.run(wallet)
  }

  fn run(self, wallet: Wallet) -> anyhow::Result<Vec<u8>> {
    ensure!(
      wallet.has_rune_index(),
      "`ord wallet mint` requires index created with `--index-runes` flag",
    );

    log::debug!("Rune index is available.");

    let rune = self.rune.rune;

    let bitcoin_client = wallet.bitcoin_client();

    // 打印获取到的比特币客户端信息
    log::debug!("Bitcoin client created.");

    let block_height = bitcoin_client.get_block_count()?;

    log::debug!("Current block height: {}", block_height);

    let Some((id, rune_entry, _)) = wallet.get_rune(rune)? else {
      bail!("rune {rune} has not been etched");
    };

    log::debug!("Rune entry found with id: {:?}", id);

    let postage = self.postage.unwrap_or(TARGET_POSTAGE);

    log::debug!("Using postage: {:?}", postage);

    let _amount = rune_entry
      .mintable(block_height)
      .map_err(|err| anyhow!("rune {rune} {err}"))?;

    log::debug!("Calculated mintable amount for rune.");

    let chain = wallet.chain();

    log::debug!("Chain selected: {:?}", chain);

    let destination = match self.destination {
      Some(destination) => destination.require_network(chain.network())?,
      None => wallet.get_change_address()?,
    };

    log::debug!("Destination address: {:?}", destination);

    ensure!(
      destination.script_pubkey().dust_value() < postage,
      "postage below dust limit of {}sat",
      destination.script_pubkey().dust_value().to_sat()
    );

    log::debug!("Postage is above dust limit.");

    let runestone = Runestone {
      mint: Some(id),
      ..default()
    };

    log::debug!("Runestone created: {:?}", runestone);

    let script_pubkey = runestone.encipher();

    ensure!(
      script_pubkey.len() <= 82,
      "runestone greater than maximum OP_RETURN size: {} > 82",
      script_pubkey.len()
    );

    log::debug!("Enciphered script pubkey within size limit.");

    let unfunded_transaction = Transaction {
      version: 2,
      lock_time: LockTime::ZERO,
      input: Vec::new(),
      output: vec![
        TxOut {
          script_pubkey,
          value: 0,
        },
        TxOut {
          script_pubkey: destination.script_pubkey(),
          value: postage.to_sat(),
        },
      ],
    };

    // 打印未资助的交易信息
    log::debug!("Unfunded transaction created: {:?}", unfunded_transaction);

    wallet.lock_non_cardinal_outputs()?;

    let unsigned_transaction =
      fund_raw_transaction(bitcoin_client, self.fee_rate, &unfunded_transaction)?;

    log::debug!("Raw transaction funded.");

    Ok(unsigned_transaction)
  }
}

#[derive(Debug)]
pub struct WalletParams {
  pub name: String,
  pub no_sync: bool,
  pub server_url: Option<Url>,
}

impl WalletParams {
  fn constructor(self) -> anyhow::Result<Wallet> {
    let options = Options {
      index_runes: true,
      ..Default::default()
    };

    log::debug!("Loading settings with options: {:?}", options);

    let settings = Settings::load(options)?;

    // 打印构造钱包时使用的服务器 URL
    log::debug!("Constructing wallet with server URL: {:?}", self.server_url);

    let wallet = WalletConstructor::construct(
      self.name.clone(),
      self.no_sync,
      settings.clone(),
      self
        .server_url
        .as_ref()
        .map(Url::as_str)
        .or(settings.server_url())
        .unwrap_or("http://127.0.0.1:80")
        .parse::<Url>()
        .context("invalid server URL")?,
    )?;

    log::debug!("Wallet constructed successfully.");

    Ok(wallet)
  }
}
