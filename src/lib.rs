use rs_zephyr_sdk::{stellar_xdr::next::{FeeBumpTransactionInnerTx, Operation, OperationBody, TransactionEnvelope, TransactionResultResult}, Condition, DatabaseDerive, DatabaseInteract, EnvClient};
use ta::{indicators::ExponentialMovingAverage, Next};

#[derive(DatabaseDerive, Clone)]
#[with_name("avgfee")]
pub struct Stats {
    pub classic: i128,
    pub contracts: i128,
    pub other: i128,
    pub fee_sor: ExponentialMovingAverage,
    pub fee_clas: ExponentialMovingAverage,
}

const PERIOD: usize = 64;


/// Please note that this design is not the most efficient (and it hasn't been thought through much) and can definitely be improved. This
/// is the result of an on-the-fly coded program in the Stellar Event "Working with Data on Stellar, the Role of Indexers and Live-Coding a ZephyrVM Program".
/// 
/// How average fees are calculated:
/// We first count the fee per transaction, basically multiply it by operation when using count_ops_and_fees, 
/// and then at the time of returning the average fees we're actually re-diving these by the number of operations to
/// get a more standardized txfee.
/// 
/// Along with the above process for both stellar "classic" and "soroban" fees, we're also tracking the amount of
/// invokehostfunction operations, stellar classic operations, and other soroban operations (ttl ops). 
#[no_mangle]
pub extern "C" fn on_close() {
    let env = EnvClient::new();

    let (contract, classic, other, avg_soroban, avg_classic) = {
        let mut contract_invocations = 0;
        let mut classic = 0;
        let mut other_soroban = 0;
        let mut tot_soroban_fee = 0;
        let mut tot_classic_fee = 0;

        for (envelope, meta) in env.reader().envelopes_with_meta() {
            let charged = meta.result.result.fee_charged;
            let success = match meta.result.result.result {
                TransactionResultResult::TxSuccess(_) => true,
                TransactionResultResult::TxFeeBumpInnerSuccess(_) => true,
                _ => false
            };

            if success {
                match envelope {
                    TransactionEnvelope::Tx(v1) => {
                        count_ops_and_fees(v1.tx.operations.to_vec(), charged, &mut classic, &mut contract_invocations, &mut other_soroban, &mut tot_soroban_fee, &mut tot_classic_fee)

                    },
                    TransactionEnvelope::TxFeeBump(feebump) => {
                        let FeeBumpTransactionInnerTx::Tx(v1) = &feebump.tx.inner_tx;
                        count_ops_and_fees(v1.tx.operations.to_vec(), charged, &mut classic, &mut contract_invocations, &mut other_soroban, &mut tot_soroban_fee, &mut tot_classic_fee)
                    },
                    TransactionEnvelope::TxV0(v0) => {
                        count_ops_and_fees(v0.tx.operations.to_vec(), charged, &mut classic, &mut contract_invocations, &mut other_soroban, &mut tot_soroban_fee, &mut tot_classic_fee)
                    }
                }
            }
        };

        (contract_invocations as i128, classic as i128, other_soroban as i128, tot_soroban_fee as f64 / (contract_invocations + other_soroban) as f64, tot_classic_fee as f64 / classic as f64)
    };

    let current = env.read::<Stats>();
    if let Some(row) = current.last() {
        let mut row = row.clone();
        if avg_classic.is_normal() {
            row.fee_clas.next(avg_classic as f64);
        };

        if avg_soroban.is_normal() {
            row.fee_sor.next(avg_soroban as f64);
        };

        let previous_classic = row.classic;

        row.classic += classic;
        row.contracts += contract;
        row.other += other;
       
        env.update(&row, &[Condition::ColumnEqualTo("classic".into(), bincode::serialize(&ZephyrVal::I128(previous_classic)).unwrap())]);
    } else {
        let mut fee_soroban = ExponentialMovingAverage::new(PERIOD).unwrap();
        fee_soroban.next(if avg_soroban.is_normal() { avg_soroban as f64 } else { 0.0 });

        let mut fee_classic = ExponentialMovingAverage::new(PERIOD).unwrap();
        fee_classic.next(if avg_classic.is_normal() { avg_classic as f64 } else { 0.0 });

        env.put(&Stats {
            classic,
            contracts: contract,
            other,
            fee_sor: fee_soroban,
            fee_clas: fee_classic
        })
    }
}

fn count_ops_and_fees(ops: Vec<Operation>, txfee: i64, classic: &mut i32, contract_invocations: &mut i32, other_soroban: &mut i32, tot_soroban_fee: &mut i64, tot_classic_fee: &mut i64) {
    for op in ops.iter() {
        match op.body {
            OperationBody::InvokeHostFunction(_) => {
                *contract_invocations += 1;
                *tot_soroban_fee += txfee;
            },
            OperationBody::ExtendFootprintTtl(_) => {
                *other_soroban += 1;
                *tot_soroban_fee += txfee;
            },
            OperationBody::RestoreFootprint(_) => {
                *other_soroban += 1;
                *tot_soroban_fee += txfee;
            },
            _ => {
                *classic += 1;
                *tot_classic_fee += txfee;
            }
        }
    }
}
