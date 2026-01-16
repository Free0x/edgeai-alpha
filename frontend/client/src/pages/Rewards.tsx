import { useState, useEffect } from 'react';
import { API_BASE_URL } from '../lib/config';

// Types
interface RewardsStats {
  pool_balance: number;
  pool_reserved: number;
  total_distributed: number;
  pending_count: number;
  pending_amount: number;
  distributed_count: number;
  unique_recipients: number;
  rewards_by_type: Record<string, number>;
  last_distribution: number;
}

interface RewardPool {
  total_balance: number;
  reserved_balance: number;
  distributed_total: number;
  last_replenishment: number;
  replenishment_rate: number;
}

interface RewardMultipliers {
  quality_low: number;
  quality_medium: number;
  quality_high: number;
  device_type_multipliers: Record<string, number>;
  region_multipliers: Record<string, number>;
  peak_hours_multiplier: number;
  off_peak_multiplier: number;
  streak_3_days: number;
  streak_7_days: number;
  streak_30_days: number;
}

interface RewardRecord {
  reward_id: string;
  recipient: string;
  reward_type: string;
  amount: number;
  status: string;
  created_at: number;
  distributed_at?: number;
  tx_hash?: string;
}

interface LeaderboardEntry {
  rank: number;
  recipient: string;
  total_rewards: number;
}

interface RewardCalculation {
  base_reward: number;
  quality_multiplier: number;
  device_type_multiplier: number;
  region_multiplier: number;
  streak_multiplier: number;
  time_multiplier: number;
  total_multiplier: number;
  final_reward: number;
}

// Reward type icons
const rewardTypeIcons: Record<string, string> = {
  DataContribution: '📊',
  QualityBonus: '⭐',
  ConsistencyBonus: '🔄',
  ScarcityBonus: '💎',
  EarlyAdopterBonus: '🌟',
  ReferralBonus: '👥',
  ValidatorReward: '✅',
  StakingReward: '🔒',
  GovernanceReward: '🗳️',
};

export default function Rewards() {
  const [stats, setStats] = useState<RewardsStats | null>(null);
  const [pool, setPool] = useState<RewardPool | null>(null);
  const [multipliers, setMultipliers] = useState<RewardMultipliers | null>(null);
  const [leaderboard, setLeaderboard] = useState<LeaderboardEntry[]>([]);
  const [recentRewards, setRecentRewards] = useState<RewardRecord[]>([]);
  const [loading, setLoading] = useState(true);
  const [activeTab, setActiveTab] = useState<'overview' | 'leaderboard' | 'calculator' | 'history'>('overview');
  
  // Calculator form
  const [calcForm, setCalcForm] = useState({
    data_bytes: 10240,
    quality_score: 0.8,
    device_type: 'TemperatureSensor',
    region: 'US',
    streak_days: 0,
  });
  const [calcResult, setCalcResult] = useState<RewardCalculation | null>(null);
  
  // Recipient lookup
  const [recipientId, setRecipientId] = useState('');
  const [recipientRewards, setRecipientRewards] = useState<any | null>(null);

  useEffect(() => {
    fetchData();
    const interval = setInterval(fetchData, 30000);
    return () => clearInterval(interval);
  }, []);

  const fetchData = async () => {
    try {
      const [statsRes, poolRes, multipliersRes, leaderboardRes, recentRes] = await Promise.all([
        fetch(`${API_BASE_URL}/api/rewards/stats`),
        fetch(`${API_BASE_URL}/api/rewards/pool`),
        fetch(`${API_BASE_URL}/api/rewards/multipliers`),
        fetch(`${API_BASE_URL}/api/rewards/leaderboard?limit=20`),
        fetch(`${API_BASE_URL}/api/rewards/recent?limit=20`),
      ]);
      
      const statsData = await statsRes.json();
      const poolData = await poolRes.json();
      const multipliersData = await multipliersRes.json();
      const leaderboardData = await leaderboardRes.json();
      const recentData = await recentRes.json();
      
      if (statsData.success) setStats(statsData.data);
      if (poolData.success) setPool(poolData.data);
      if (multipliersData.success) setMultipliers(multipliersData.data);
      if (leaderboardData.success) setLeaderboard(leaderboardData.data);
      if (recentData.success) setRecentRewards(recentData.data);
    } catch (error) {
      console.error('Failed to fetch rewards data:', error);
    } finally {
      setLoading(false);
    }
  };

  const handleCalculate = async () => {
    try {
      const response = await fetch(`${API_BASE_URL}/api/rewards/calculate`, {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify(calcForm),
      });
      
      const data = await response.json();
      if (data.success) {
        setCalcResult(data.data);
      }
    } catch (error) {
      console.error('Failed to calculate reward:', error);
    }
  };

  const handleLookupRecipient = async () => {
    if (!recipientId) return;
    
    try {
      const response = await fetch(`${API_BASE_URL}/api/rewards/recipient/${recipientId}`);
      const data = await response.json();
      if (data.success) {
        setRecipientRewards(data.data);
      }
    } catch (error) {
      console.error('Failed to lookup recipient:', error);
    }
  };

  const formatTime = (timestamp: number) => {
    if (!timestamp) return 'N/A';
    return new Date(timestamp * 1000).toLocaleString();
  };

  const formatNumber = (num: number, decimals = 2) => {
    if (num >= 1000000) return `${(num / 1000000).toFixed(decimals)}M`;
    if (num >= 1000) return `${(num / 1000).toFixed(decimals)}K`;
    return num.toFixed(decimals);
  };

  if (loading) {
    return (
      <div className="flex items-center justify-center min-h-screen">
        <div className="animate-spin rounded-full h-12 w-12 border-b-2 border-cyan-500"></div>
      </div>
    );
  }

  return (
    <div className="min-h-screen bg-gray-900 text-white p-6">
      <div className="max-w-7xl mx-auto">
        {/* Header */}
        <div className="mb-8">
          <h1 className="text-3xl font-bold mb-2">🎁 Rewards Center</h1>
          <p className="text-gray-400">Track rewards, view leaderboard, and calculate earnings</p>
        </div>

        {/* Tabs */}
        <div className="flex space-x-4 mb-6 border-b border-gray-700 pb-2">
          {[
            { id: 'overview', label: '📊 Overview' },
            { id: 'leaderboard', label: '🏆 Leaderboard' },
            { id: 'calculator', label: '🧮 Calculator' },
            { id: 'history', label: '📜 History' },
          ].map((tab) => (
            <button
              key={tab.id}
              onClick={() => setActiveTab(tab.id as any)}
              className={`px-4 py-2 rounded-t-lg transition-colors ${
                activeTab === tab.id
                  ? 'bg-cyan-600 text-white'
                  : 'text-gray-400 hover:text-white hover:bg-gray-800'
              }`}
            >
              {tab.label}
            </button>
          ))}
        </div>

        {/* Overview Tab */}
        {activeTab === 'overview' && stats && pool && (
          <div className="space-y-6">
            {/* Pool Stats */}
            <div className="grid grid-cols-2 md:grid-cols-4 gap-4">
              <div className="bg-gradient-to-br from-cyan-900 to-cyan-800 rounded-xl p-4">
                <div className="text-cyan-300 text-sm">Pool Balance</div>
                <div className="text-2xl font-bold">{formatNumber(pool.total_balance)} EDGE</div>
              </div>
              <div className="bg-gradient-to-br from-green-900 to-green-800 rounded-xl p-4">
                <div className="text-green-300 text-sm">Total Distributed</div>
                <div className="text-2xl font-bold">{formatNumber(stats.total_distributed)} EDGE</div>
              </div>
              <div className="bg-gradient-to-br from-purple-900 to-purple-800 rounded-xl p-4">
                <div className="text-purple-300 text-sm">Unique Recipients</div>
                <div className="text-2xl font-bold">{stats.unique_recipients.toLocaleString()}</div>
              </div>
              <div className="bg-gradient-to-br from-yellow-900 to-yellow-800 rounded-xl p-4">
                <div className="text-yellow-300 text-sm">Pending Rewards</div>
                <div className="text-2xl font-bold">{formatNumber(stats.pending_amount)} EDGE</div>
              </div>
            </div>

            {/* Pool Details */}
            <div className="bg-gray-800 rounded-xl p-6">
              <h3 className="text-lg font-semibold mb-4">Reward Pool Status</h3>
              <div className="grid grid-cols-2 md:grid-cols-4 gap-4">
                <div>
                  <div className="text-gray-400 text-sm">Available</div>
                  <div className="text-lg font-semibold text-green-400">
                    {formatNumber(pool.total_balance - pool.reserved_balance)} EDGE
                  </div>
                </div>
                <div>
                  <div className="text-gray-400 text-sm">Reserved</div>
                  <div className="text-lg font-semibold text-yellow-400">
                    {formatNumber(pool.reserved_balance)} EDGE
                  </div>
                </div>
                <div>
                  <div className="text-gray-400 text-sm">Replenishment Rate</div>
                  <div className="text-lg font-semibold text-cyan-400">
                    {pool.replenishment_rate} EDGE/block
                  </div>
                </div>
                <div>
                  <div className="text-gray-400 text-sm">Last Distribution</div>
                  <div className="text-lg font-semibold">
                    {formatTime(stats.last_distribution)}
                  </div>
                </div>
              </div>
              
              {/* Pool Progress Bar */}
              <div className="mt-4">
                <div className="flex justify-between text-sm text-gray-400 mb-1">
                  <span>Pool Utilization</span>
                  <span>{((pool.reserved_balance / pool.total_balance) * 100).toFixed(1)}%</span>
                </div>
                <div className="w-full bg-gray-700 rounded-full h-3">
                  <div 
                    className="bg-gradient-to-r from-cyan-500 to-green-500 h-3 rounded-full transition-all"
                    style={{ width: `${(pool.reserved_balance / pool.total_balance) * 100}%` }}
                  ></div>
                </div>
              </div>
            </div>

            {/* Rewards by Type */}
            <div className="bg-gray-800 rounded-xl p-6">
              <h3 className="text-lg font-semibold mb-4">Rewards by Type</h3>
              <div className="grid grid-cols-2 md:grid-cols-3 gap-3">
                {Object.entries(stats.rewards_by_type).map(([type, amount]) => (
                  <div key={type} className="bg-gray-700 rounded-lg p-3 flex items-center space-x-3">
                    <span className="text-2xl">{rewardTypeIcons[type] || '🎁'}</span>
                    <div>
                      <div className="text-sm text-gray-400">{type.replace(/([A-Z])/g, ' $1').trim()}</div>
                      <div className="font-semibold text-cyan-400">{formatNumber(amount)} EDGE</div>
                    </div>
                  </div>
                ))}
              </div>
            </div>

            {/* Multipliers */}
            {multipliers && (
              <div className="bg-gray-800 rounded-xl p-6">
                <h3 className="text-lg font-semibold mb-4">Active Multipliers</h3>
                <div className="grid grid-cols-2 md:grid-cols-4 gap-4">
                  <div className="bg-gray-700 rounded-lg p-3">
                    <div className="text-sm text-gray-400">Quality (High)</div>
                    <div className="text-lg font-semibold text-green-400">{multipliers.quality_high}x</div>
                  </div>
                  <div className="bg-gray-700 rounded-lg p-3">
                    <div className="text-sm text-gray-400">30-Day Streak</div>
                    <div className="text-lg font-semibold text-purple-400">{multipliers.streak_30_days}x</div>
                  </div>
                  <div className="bg-gray-700 rounded-lg p-3">
                    <div className="text-sm text-gray-400">Off-Peak Hours</div>
                    <div className="text-lg font-semibold text-cyan-400">{multipliers.off_peak_multiplier}x</div>
                  </div>
                  <div className="bg-gray-700 rounded-lg p-3">
                    <div className="text-sm text-gray-400">Edge AI Device</div>
                    <div className="text-lg font-semibold text-yellow-400">
                      {multipliers.device_type_multipliers['EdgeAIProcessor'] || 2.0}x
                    </div>
                  </div>
                </div>
              </div>
            )}
          </div>
        )}

        {/* Leaderboard Tab */}
        {activeTab === 'leaderboard' && (
          <div className="bg-gray-800 rounded-xl p-6">
            <h3 className="text-lg font-semibold mb-4">Top Earners</h3>
            <div className="overflow-x-auto">
              <table className="w-full">
                <thead>
                  <tr className="text-gray-400 text-left">
                    <th className="pb-3">Rank</th>
                    <th className="pb-3">Recipient</th>
                    <th className="pb-3 text-right">Total Rewards</th>
                  </tr>
                </thead>
                <tbody>
                  {leaderboard.map((entry) => (
                    <tr key={entry.recipient} className="border-t border-gray-700 hover:bg-gray-700/50">
                      <td className="py-3">
                        <span className={`text-2xl ${
                          entry.rank === 1 ? 'text-yellow-400' :
                          entry.rank === 2 ? 'text-gray-300' :
                          entry.rank === 3 ? 'text-amber-600' :
                          'text-gray-500'
                        }`}>
                          {entry.rank === 1 ? '🥇' : entry.rank === 2 ? '🥈' : entry.rank === 3 ? '🥉' : `#${entry.rank}`}
                        </span>
                      </td>
                      <td className="py-3 font-mono text-sm">{entry.recipient}</td>
                      <td className="py-3 text-right text-cyan-400 font-semibold">
                        {formatNumber(entry.total_rewards)} EDGE
                      </td>
                    </tr>
                  ))}
                </tbody>
              </table>
            </div>
          </div>
        )}

        {/* Calculator Tab */}
        {activeTab === 'calculator' && (
          <div className="grid md:grid-cols-2 gap-6">
            <div className="bg-gray-800 rounded-xl p-6">
              <h3 className="text-lg font-semibold mb-4">Reward Calculator</h3>
              <div className="space-y-4">
                <div>
                  <label className="block text-sm text-gray-400 mb-1">Data Size (bytes)</label>
                  <input
                    type="number"
                    value={calcForm.data_bytes}
                    onChange={(e) => setCalcForm({ ...calcForm, data_bytes: parseInt(e.target.value) || 0 })}
                    className="w-full bg-gray-700 rounded-lg px-4 py-2 focus:outline-none focus:ring-2 focus:ring-cyan-500"
                  />
                </div>
                
                <div>
                  <label className="block text-sm text-gray-400 mb-1">Quality Score (0-1)</label>
                  <input
                    type="range"
                    min="0"
                    max="1"
                    step="0.1"
                    value={calcForm.quality_score}
                    onChange={(e) => setCalcForm({ ...calcForm, quality_score: parseFloat(e.target.value) })}
                    className="w-full"
                  />
                  <div className="text-center text-cyan-400">{calcForm.quality_score}</div>
                </div>
                
                <div>
                  <label className="block text-sm text-gray-400 mb-1">Device Type</label>
                  <select
                    value={calcForm.device_type}
                    onChange={(e) => setCalcForm({ ...calcForm, device_type: e.target.value })}
                    className="w-full bg-gray-700 rounded-lg px-4 py-2 focus:outline-none focus:ring-2 focus:ring-cyan-500"
                  >
                    <option value="TemperatureSensor">Temperature Sensor</option>
                    <option value="Camera">Camera</option>
                    <option value="IndustrialSensor">Industrial Sensor</option>
                    <option value="MedicalDevice">Medical Device</option>
                    <option value="EdgeAIProcessor">Edge AI Processor</option>
                    <option value="WeatherStation">Weather Station</option>
                  </select>
                </div>
                
                <div>
                  <label className="block text-sm text-gray-400 mb-1">Region</label>
                  <select
                    value={calcForm.region}
                    onChange={(e) => setCalcForm({ ...calcForm, region: e.target.value })}
                    className="w-full bg-gray-700 rounded-lg px-4 py-2 focus:outline-none focus:ring-2 focus:ring-cyan-500"
                  >
                    <option value="US">United States</option>
                    <option value="EU">Europe</option>
                    <option value="AS">Asia</option>
                    <option value="AF">Africa (2x bonus)</option>
                    <option value="SA">South America (1.8x bonus)</option>
                    <option value="OC">Oceania (1.5x bonus)</option>
                  </select>
                </div>
                
                <div>
                  <label className="block text-sm text-gray-400 mb-1">Contribution Streak (days)</label>
                  <input
                    type="number"
                    value={calcForm.streak_days}
                    onChange={(e) => setCalcForm({ ...calcForm, streak_days: parseInt(e.target.value) || 0 })}
                    min="0"
                    max="365"
                    className="w-full bg-gray-700 rounded-lg px-4 py-2 focus:outline-none focus:ring-2 focus:ring-cyan-500"
                  />
                </div>
                
                <button
                  onClick={handleCalculate}
                  className="w-full bg-cyan-600 hover:bg-cyan-700 text-white font-semibold py-3 rounded-lg transition-colors"
                >
                  Calculate Reward
                </button>
              </div>
            </div>
            
            {/* Calculation Result */}
            <div className="bg-gray-800 rounded-xl p-6">
              <h3 className="text-lg font-semibold mb-4">Calculation Result</h3>
              {calcResult ? (
                <div className="space-y-4">
                  <div className="bg-gradient-to-r from-cyan-900 to-green-900 rounded-xl p-6 text-center">
                    <div className="text-gray-300 text-sm">Estimated Reward</div>
                    <div className="text-4xl font-bold text-white">{calcResult.final_reward.toFixed(4)} EDGE</div>
                  </div>
                  
                  <div className="space-y-2">
                    <div className="flex justify-between">
                      <span className="text-gray-400">Base Reward</span>
                      <span>{calcResult.base_reward.toFixed(4)} EDGE</span>
                    </div>
                    <div className="flex justify-between">
                      <span className="text-gray-400">Quality Multiplier</span>
                      <span className={calcResult.quality_multiplier > 1 ? 'text-green-400' : 'text-yellow-400'}>
                        {calcResult.quality_multiplier.toFixed(2)}x
                      </span>
                    </div>
                    <div className="flex justify-between">
                      <span className="text-gray-400">Device Type Multiplier</span>
                      <span className={calcResult.device_type_multiplier > 1 ? 'text-green-400' : ''}>
                        {calcResult.device_type_multiplier.toFixed(2)}x
                      </span>
                    </div>
                    <div className="flex justify-between">
                      <span className="text-gray-400">Region Multiplier</span>
                      <span className={calcResult.region_multiplier > 1 ? 'text-green-400' : ''}>
                        {calcResult.region_multiplier.toFixed(2)}x
                      </span>
                    </div>
                    <div className="flex justify-between">
                      <span className="text-gray-400">Streak Multiplier</span>
                      <span className={calcResult.streak_multiplier > 1 ? 'text-purple-400' : ''}>
                        {calcResult.streak_multiplier.toFixed(2)}x
                      </span>
                    </div>
                    <div className="flex justify-between">
                      <span className="text-gray-400">Time Multiplier</span>
                      <span className={calcResult.time_multiplier > 1 ? 'text-cyan-400' : 'text-yellow-400'}>
                        {calcResult.time_multiplier.toFixed(2)}x
                      </span>
                    </div>
                    <hr className="border-gray-700" />
                    <div className="flex justify-between font-semibold">
                      <span>Total Multiplier</span>
                      <span className="text-cyan-400">{calcResult.total_multiplier.toFixed(2)}x</span>
                    </div>
                  </div>
                </div>
              ) : (
                <div className="text-center text-gray-500 py-12">
                  Enter parameters and click Calculate to see estimated rewards
                </div>
              )}
            </div>
          </div>
        )}

        {/* History Tab */}
        {activeTab === 'history' && (
          <div className="space-y-6">
            {/* Recipient Lookup */}
            <div className="bg-gray-800 rounded-xl p-6">
              <h3 className="text-lg font-semibold mb-4">Lookup Recipient Rewards</h3>
              <div className="flex space-x-4">
                <input
                  type="text"
                  value={recipientId}
                  onChange={(e) => setRecipientId(e.target.value)}
                  placeholder="Enter device ID or wallet address"
                  className="flex-1 bg-gray-700 rounded-lg px-4 py-2 focus:outline-none focus:ring-2 focus:ring-cyan-500"
                />
                <button
                  onClick={handleLookupRecipient}
                  className="bg-cyan-600 hover:bg-cyan-700 px-6 py-2 rounded-lg transition-colors"
                >
                  Lookup
                </button>
              </div>
              
              {recipientRewards && (
                <div className="mt-4 bg-gray-700 rounded-lg p-4">
                  <div className="grid grid-cols-2 md:grid-cols-4 gap-4 mb-4">
                    <div>
                      <div className="text-gray-400 text-sm">Total Earned</div>
                      <div className="text-lg font-semibold text-cyan-400">
                        {formatNumber(recipientRewards.total_earned)} EDGE
                      </div>
                    </div>
                    <div>
                      <div className="text-gray-400 text-sm">Pending</div>
                      <div className="text-lg font-semibold text-yellow-400">
                        {formatNumber(recipientRewards.pending_amount)} EDGE
                      </div>
                    </div>
                    <div>
                      <div className="text-gray-400 text-sm">Pending Count</div>
                      <div className="text-lg font-semibold">{recipientRewards.pending_count}</div>
                    </div>
                    <div>
                      <div className="text-gray-400 text-sm">Distributed Count</div>
                      <div className="text-lg font-semibold">{recipientRewards.distributed_count}</div>
                    </div>
                  </div>
                </div>
              )}
            </div>

            {/* Recent Rewards */}
            <div className="bg-gray-800 rounded-xl p-6">
              <h3 className="text-lg font-semibold mb-4">Recent Rewards</h3>
              <div className="overflow-x-auto">
                <table className="w-full">
                  <thead>
                    <tr className="text-gray-400 text-left">
                      <th className="pb-3">Recipient</th>
                      <th className="pb-3">Type</th>
                      <th className="pb-3">Amount</th>
                      <th className="pb-3">Status</th>
                      <th className="pb-3">Time</th>
                    </tr>
                  </thead>
                  <tbody>
                    {recentRewards.map((reward) => (
                      <tr key={reward.reward_id} className="border-t border-gray-700 hover:bg-gray-700/50">
                        <td className="py-3 font-mono text-sm">{reward.recipient.slice(0, 20)}...</td>
                        <td className="py-3">
                          <span className="flex items-center space-x-2">
                            <span>{rewardTypeIcons[reward.reward_type] || '🎁'}</span>
                            <span>{reward.reward_type.replace(/([A-Z])/g, ' $1').trim()}</span>
                          </span>
                        </td>
                        <td className="py-3 text-cyan-400 font-semibold">{reward.amount.toFixed(4)} EDGE</td>
                        <td className="py-3">
                          <span className={`px-2 py-1 rounded text-xs ${
                            reward.status === 'Distributed' ? 'bg-green-900 text-green-300' :
                            reward.status === 'Pending' ? 'bg-yellow-900 text-yellow-300' :
                            reward.status === 'Claimed' ? 'bg-cyan-900 text-cyan-300' :
                            'bg-gray-700 text-gray-300'
                          }`}>
                            {reward.status}
                          </span>
                        </td>
                        <td className="py-3 text-gray-400 text-sm">{formatTime(reward.created_at)}</td>
                      </tr>
                    ))}
                  </tbody>
                </table>
              </div>
            </div>
          </div>
        )}
      </div>
    </div>
  );
}
