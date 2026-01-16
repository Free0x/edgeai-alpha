import { useState, useEffect } from 'react';
import { API_BASE_URL } from '../lib/config';

// Types
interface GeoLocation {
  country_code: string;
  region_code?: string;
  city?: string;
  latitude?: number;
  longitude?: number;
}

interface DeviceCapabilities {
  data_types: string[];
  sampling_rate_hz?: number;
  accuracy?: number;
  resolution?: string;
  connectivity: string[];
  edge_compute: boolean;
  storage_mb?: number;
  battery_powered: boolean;
}

interface IoTDevice {
  device_id: string;
  owner_address: string;
  device_type: string;
  status: string;
  location: GeoLocation;
  capabilities: DeviceCapabilities;
  is_online: boolean;
  total_contributions: number;
  total_data_bytes: number;
  contribution_points: number;
  reputation_score: number;
  quality_score_avg: number;
  pending_rewards: number;
  total_rewards_earned: number;
  reward_multiplier: number;
  registered_at: number;
  last_seen_at: number;
  verified_at?: number;
}

interface IoTStats {
  total_devices: number;
  active_devices: number;
  online_devices: number;
  verified_devices: number;
  total_contributions: number;
  total_data_bytes: number;
  total_rewards_distributed: number;
  device_type_distribution: Record<string, number>;
}

interface Contribution {
  contribution_id: string;
  device_id: string;
  data_type: string;
  data_hash: string;
  data_size_bytes: number;
  quality_score: number;
  total_reward: number;
  timestamp: number;
}

// Device type icons
const deviceTypeIcons: Record<string, string> = {
  TemperatureSensor: '🌡️',
  HumiditySensor: '💧',
  AirQualitySensor: '🌬️',
  WeatherStation: '⛅',
  Camera: '📷',
  Microphone: '🎤',
  LidarSensor: '📡',
  GpsTracker: '📍',
  MotionSensor: '🏃',
  IndustrialSensor: '🏭',
  EnergyMeter: '⚡',
  SmartThermostat: '🏠',
  HealthMonitor: '❤️',
  VehicleTelematics: '🚗',
  DroneController: '🚁',
  SoilSensor: '🌱',
  MedicalDevice: '🏥',
  EdgeAIProcessor: '🤖',
  NeuralAccelerator: '🧠',
  Custom: '📦',
};

// Device types for registration
const deviceTypes = [
  { value: 'temperature_sensor', label: 'Temperature Sensor', icon: '🌡️' },
  { value: 'humidity_sensor', label: 'Humidity Sensor', icon: '💧' },
  { value: 'air_quality_sensor', label: 'Air Quality Sensor', icon: '🌬️' },
  { value: 'weather_station', label: 'Weather Station', icon: '⛅' },
  { value: 'camera', label: 'Camera', icon: '📷' },
  { value: 'microphone', label: 'Microphone', icon: '🎤' },
  { value: 'gps_tracker', label: 'GPS Tracker', icon: '📍' },
  { value: 'motion_sensor', label: 'Motion Sensor', icon: '🏃' },
  { value: 'industrial_sensor', label: 'Industrial Sensor', icon: '🏭' },
  { value: 'energy_meter', label: 'Energy Meter', icon: '⚡' },
  { value: 'smart_thermostat', label: 'Smart Thermostat', icon: '🏠' },
  { value: 'health_monitor', label: 'Health Monitor', icon: '❤️' },
  { value: 'vehicle_telematics', label: 'Vehicle Telematics', icon: '🚗' },
  { value: 'soil_sensor', label: 'Soil Sensor', icon: '🌱' },
  { value: 'medical_device', label: 'Medical Device', icon: '🏥' },
  { value: 'edge_ai', label: 'Edge AI Processor', icon: '🤖' },
];

export default function IoT() {
  const [stats, setStats] = useState<IoTStats | null>(null);
  const [devices, setDevices] = useState<IoTDevice[]>([]);
  const [contributions, setContributions] = useState<Contribution[]>([]);
  const [loading, setLoading] = useState(true);
  const [activeTab, setActiveTab] = useState<'overview' | 'devices' | 'register' | 'contribute'>('overview');
  
  // Registration form
  const [regForm, setRegForm] = useState({
    owner_address: '',
    public_key: '',
    device_type: 'temperature_sensor',
    country_code: 'US',
    latitude: '',
    longitude: '',
    edge_compute: false,
  });
  
  // Contribution form
  const [contribForm, setContribForm] = useState({
    device_id: '',
    data_type: 'sensor_data',
    data_hash: '',
    data_size_bytes: 1024,
  });
  
  const [message, setMessage] = useState<{ type: 'success' | 'error'; text: string } | null>(null);

  useEffect(() => {
    fetchData();
    const interval = setInterval(fetchData, 30000);
    return () => clearInterval(interval);
  }, []);

  const fetchData = async () => {
    try {
      const [statsRes, devicesRes, contribRes] = await Promise.all([
        fetch(`${API_BASE_URL}/api/iot/devices`),
        fetch(`${API_BASE_URL}/api/iot/devices/leaderboard?limit=20`),
        fetch(`${API_BASE_URL}/api/iot/data/recent?limit=20`),
      ]);
      
      const statsData = await statsRes.json();
      const devicesData = await devicesRes.json();
      const contribData = await contribRes.json();
      
      if (statsData.success) setStats(statsData.data);
      if (devicesData.success) setDevices(devicesData.data);
      if (contribData.success) setContributions(contribData.data);
    } catch (error) {
      console.error('Failed to fetch IoT data:', error);
    } finally {
      setLoading(false);
    }
  };

  const handleRegister = async (e: React.FormEvent) => {
    e.preventDefault();
    setMessage(null);
    
    try {
      const response = await fetch(`${API_BASE_URL}/api/iot/devices/register`, {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({
          owner_address: regForm.owner_address,
          public_key: regForm.public_key || `pk_${Date.now()}`,
          device_type: regForm.device_type,
          location: {
            country_code: regForm.country_code,
            latitude: regForm.latitude ? parseFloat(regForm.latitude) : undefined,
            longitude: regForm.longitude ? parseFloat(regForm.longitude) : undefined,
          },
          capabilities: {
            data_types: ['sensor_data'],
            connectivity: ['wifi'],
            edge_compute: regForm.edge_compute,
            battery_powered: false,
          },
        }),
      });
      
      const data = await response.json();
      
      if (data.success) {
        setMessage({ type: 'success', text: `Device registered! ID: ${data.data.device_id}` });
        fetchData();
        setRegForm({
          owner_address: '',
          public_key: '',
          device_type: 'temperature_sensor',
          country_code: 'US',
          latitude: '',
          longitude: '',
          edge_compute: false,
        });
      } else {
        setMessage({ type: 'error', text: data.error || 'Registration failed' });
      }
    } catch (error) {
      setMessage({ type: 'error', text: 'Failed to register device' });
    }
  };

  const handleContribute = async (e: React.FormEvent) => {
    e.preventDefault();
    setMessage(null);
    
    try {
      const response = await fetch(`${API_BASE_URL}/api/iot/data/submit`, {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({
          device_id: contribForm.device_id,
          signature: 'demo_signature',
          data_type: contribForm.data_type,
          data_hash: contribForm.data_hash || `hash_${Date.now()}`,
          data_size_bytes: contribForm.data_size_bytes,
        }),
      });
      
      const data = await response.json();
      
      if (data.success) {
        setMessage({ 
          type: 'success', 
          text: `Data submitted! Reward: ${data.data.total_reward.toFixed(4)} EDGE` 
        });
        fetchData();
      } else {
        setMessage({ type: 'error', text: data.error || 'Submission failed' });
      }
    } catch (error) {
      setMessage({ type: 'error', text: 'Failed to submit data' });
    }
  };

  const formatBytes = (bytes: number) => {
    if (bytes < 1024) return `${bytes} B`;
    if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)} KB`;
    if (bytes < 1024 * 1024 * 1024) return `${(bytes / 1024 / 1024).toFixed(1)} MB`;
    return `${(bytes / 1024 / 1024 / 1024).toFixed(2)} GB`;
  };

  const formatTime = (timestamp: number) => {
    return new Date(timestamp * 1000).toLocaleString();
  };

  const getStatusColor = (status: string) => {
    switch (status) {
      case 'Active': return 'bg-green-500';
      case 'Pending': return 'bg-yellow-500';
      case 'Inactive': return 'bg-gray-500';
      case 'Suspended': return 'bg-orange-500';
      case 'Banned': return 'bg-red-500';
      default: return 'bg-gray-500';
    }
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
          <h1 className="text-3xl font-bold mb-2">🌐 IoT Device Hub</h1>
          <p className="text-gray-400">Register devices, contribute data, and earn rewards</p>
        </div>

        {/* Tabs */}
        <div className="flex space-x-4 mb-6 border-b border-gray-700 pb-2">
          {[
            { id: 'overview', label: '📊 Overview' },
            { id: 'devices', label: '📱 Devices' },
            { id: 'register', label: '➕ Register' },
            { id: 'contribute', label: '📤 Contribute' },
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

        {/* Message */}
        {message && (
          <div className={`mb-6 p-4 rounded-lg ${
            message.type === 'success' ? 'bg-green-900/50 border border-green-500' : 'bg-red-900/50 border border-red-500'
          }`}>
            {message.text}
          </div>
        )}

        {/* Overview Tab */}
        {activeTab === 'overview' && stats && (
          <div className="space-y-6">
            {/* Stats Grid */}
            <div className="grid grid-cols-2 md:grid-cols-4 gap-4">
              <div className="bg-gray-800 rounded-xl p-4">
                <div className="text-gray-400 text-sm">Total Devices</div>
                <div className="text-2xl font-bold text-cyan-400">{stats.total_devices.toLocaleString()}</div>
              </div>
              <div className="bg-gray-800 rounded-xl p-4">
                <div className="text-gray-400 text-sm">Online Now</div>
                <div className="text-2xl font-bold text-green-400">{stats.online_devices.toLocaleString()}</div>
              </div>
              <div className="bg-gray-800 rounded-xl p-4">
                <div className="text-gray-400 text-sm">Total Contributions</div>
                <div className="text-2xl font-bold text-purple-400">{stats.total_contributions.toLocaleString()}</div>
              </div>
              <div className="bg-gray-800 rounded-xl p-4">
                <div className="text-gray-400 text-sm">Data Contributed</div>
                <div className="text-2xl font-bold text-yellow-400">{formatBytes(stats.total_data_bytes)}</div>
              </div>
            </div>

            {/* Device Type Distribution */}
            <div className="bg-gray-800 rounded-xl p-6">
              <h3 className="text-lg font-semibold mb-4">Device Type Distribution</h3>
              <div className="grid grid-cols-2 md:grid-cols-4 gap-3">
                {Object.entries(stats.device_type_distribution).map(([type, count]) => (
                  <div key={type} className="bg-gray-700 rounded-lg p-3 flex items-center space-x-3">
                    <span className="text-2xl">{deviceTypeIcons[type] || '📦'}</span>
                    <div>
                      <div className="text-sm text-gray-400">{type.replace(/([A-Z])/g, ' $1').trim()}</div>
                      <div className="font-semibold">{count}</div>
                    </div>
                  </div>
                ))}
              </div>
            </div>

            {/* Recent Contributions */}
            <div className="bg-gray-800 rounded-xl p-6">
              <h3 className="text-lg font-semibold mb-4">Recent Contributions</h3>
              <div className="overflow-x-auto">
                <table className="w-full">
                  <thead>
                    <tr className="text-gray-400 text-left">
                      <th className="pb-3">Device</th>
                      <th className="pb-3">Type</th>
                      <th className="pb-3">Size</th>
                      <th className="pb-3">Quality</th>
                      <th className="pb-3">Reward</th>
                      <th className="pb-3">Time</th>
                    </tr>
                  </thead>
                  <tbody>
                    {contributions.slice(0, 10).map((c) => (
                      <tr key={c.contribution_id} className="border-t border-gray-700">
                        <td className="py-3 font-mono text-sm">{c.device_id.slice(0, 16)}...</td>
                        <td className="py-3">{c.data_type}</td>
                        <td className="py-3">{formatBytes(c.data_size_bytes)}</td>
                        <td className="py-3">
                          <span className={`px-2 py-1 rounded text-xs ${
                            c.quality_score >= 0.7 ? 'bg-green-900 text-green-300' :
                            c.quality_score >= 0.4 ? 'bg-yellow-900 text-yellow-300' :
                            'bg-red-900 text-red-300'
                          }`}>
                            {(c.quality_score * 100).toFixed(0)}%
                          </span>
                        </td>
                        <td className="py-3 text-cyan-400">{c.total_reward.toFixed(4)} EDGE</td>
                        <td className="py-3 text-gray-400 text-sm">{formatTime(c.timestamp)}</td>
                      </tr>
                    ))}
                  </tbody>
                </table>
              </div>
            </div>
          </div>
        )}

        {/* Devices Tab */}
        {activeTab === 'devices' && (
          <div className="bg-gray-800 rounded-xl p-6">
            <h3 className="text-lg font-semibold mb-4">Device Leaderboard</h3>
            <div className="overflow-x-auto">
              <table className="w-full">
                <thead>
                  <tr className="text-gray-400 text-left">
                    <th className="pb-3">#</th>
                    <th className="pb-3">Device</th>
                    <th className="pb-3">Type</th>
                    <th className="pb-3">Status</th>
                    <th className="pb-3">Reputation</th>
                    <th className="pb-3">Contributions</th>
                    <th className="pb-3">Rewards Earned</th>
                    <th className="pb-3">Multiplier</th>
                  </tr>
                </thead>
                <tbody>
                  {devices.map((device, index) => (
                    <tr key={device.device_id} className="border-t border-gray-700 hover:bg-gray-700/50">
                      <td className="py-3 text-gray-400">{index + 1}</td>
                      <td className="py-3">
                        <div className="font-mono text-sm">{device.device_id.slice(0, 20)}...</div>
                        <div className="text-xs text-gray-500">{device.location.country_code}</div>
                      </td>
                      <td className="py-3">
                        <span className="flex items-center space-x-2">
                          <span>{deviceTypeIcons[device.device_type] || '📦'}</span>
                          <span>{device.device_type.replace(/([A-Z])/g, ' $1').trim()}</span>
                        </span>
                      </td>
                      <td className="py-3">
                        <span className={`px-2 py-1 rounded text-xs ${getStatusColor(device.status)} bg-opacity-20`}>
                          {device.is_online ? '🟢' : '⚫'} {device.status}
                        </span>
                      </td>
                      <td className="py-3">
                        <div className="flex items-center space-x-2">
                          <div className="w-16 bg-gray-700 rounded-full h-2">
                            <div 
                              className="bg-cyan-500 h-2 rounded-full" 
                              style={{ width: `${device.reputation_score}%` }}
                            ></div>
                          </div>
                          <span className="text-sm">{device.reputation_score.toFixed(0)}</span>
                        </div>
                      </td>
                      <td className="py-3">{device.total_contributions.toLocaleString()}</td>
                      <td className="py-3 text-cyan-400">{device.total_rewards_earned.toFixed(2)} EDGE</td>
                      <td className="py-3 text-yellow-400">{device.reward_multiplier.toFixed(2)}x</td>
                    </tr>
                  ))}
                </tbody>
              </table>
            </div>
          </div>
        )}

        {/* Register Tab */}
        {activeTab === 'register' && (
          <div className="bg-gray-800 rounded-xl p-6 max-w-2xl">
            <h3 className="text-lg font-semibold mb-4">Register New Device</h3>
            <form onSubmit={handleRegister} className="space-y-4">
              <div>
                <label className="block text-sm text-gray-400 mb-1">Owner Address *</label>
                <input
                  type="text"
                  value={regForm.owner_address}
                  onChange={(e) => setRegForm({ ...regForm, owner_address: e.target.value })}
                  placeholder="edge1..."
                  className="w-full bg-gray-700 rounded-lg px-4 py-2 focus:outline-none focus:ring-2 focus:ring-cyan-500"
                  required
                />
              </div>
              
              <div>
                <label className="block text-sm text-gray-400 mb-1">Device Type *</label>
                <select
                  value={regForm.device_type}
                  onChange={(e) => setRegForm({ ...regForm, device_type: e.target.value })}
                  className="w-full bg-gray-700 rounded-lg px-4 py-2 focus:outline-none focus:ring-2 focus:ring-cyan-500"
                >
                  {deviceTypes.map((type) => (
                    <option key={type.value} value={type.value}>
                      {type.icon} {type.label}
                    </option>
                  ))}
                </select>
              </div>
              
              <div className="grid grid-cols-2 gap-4">
                <div>
                  <label className="block text-sm text-gray-400 mb-1">Country Code *</label>
                  <input
                    type="text"
                    value={regForm.country_code}
                    onChange={(e) => setRegForm({ ...regForm, country_code: e.target.value.toUpperCase() })}
                    placeholder="US"
                    maxLength={2}
                    className="w-full bg-gray-700 rounded-lg px-4 py-2 focus:outline-none focus:ring-2 focus:ring-cyan-500"
                    required
                  />
                </div>
                <div>
                  <label className="block text-sm text-gray-400 mb-1">Public Key (optional)</label>
                  <input
                    type="text"
                    value={regForm.public_key}
                    onChange={(e) => setRegForm({ ...regForm, public_key: e.target.value })}
                    placeholder="Auto-generated if empty"
                    className="w-full bg-gray-700 rounded-lg px-4 py-2 focus:outline-none focus:ring-2 focus:ring-cyan-500"
                  />
                </div>
              </div>
              
              <div className="grid grid-cols-2 gap-4">
                <div>
                  <label className="block text-sm text-gray-400 mb-1">Latitude (optional)</label>
                  <input
                    type="number"
                    step="0.000001"
                    value={regForm.latitude}
                    onChange={(e) => setRegForm({ ...regForm, latitude: e.target.value })}
                    placeholder="-90 to 90"
                    className="w-full bg-gray-700 rounded-lg px-4 py-2 focus:outline-none focus:ring-2 focus:ring-cyan-500"
                  />
                </div>
                <div>
                  <label className="block text-sm text-gray-400 mb-1">Longitude (optional)</label>
                  <input
                    type="number"
                    step="0.000001"
                    value={regForm.longitude}
                    onChange={(e) => setRegForm({ ...regForm, longitude: e.target.value })}
                    placeholder="-180 to 180"
                    className="w-full bg-gray-700 rounded-lg px-4 py-2 focus:outline-none focus:ring-2 focus:ring-cyan-500"
                  />
                </div>
              </div>
              
              <div className="flex items-center space-x-2">
                <input
                  type="checkbox"
                  id="edge_compute"
                  checked={regForm.edge_compute}
                  onChange={(e) => setRegForm({ ...regForm, edge_compute: e.target.checked })}
                  className="rounded bg-gray-700 border-gray-600 text-cyan-500 focus:ring-cyan-500"
                />
                <label htmlFor="edge_compute" className="text-sm text-gray-400">
                  Device supports Edge AI compute (1.2x reward bonus)
                </label>
              </div>
              
              <button
                type="submit"
                className="w-full bg-cyan-600 hover:bg-cyan-700 text-white font-semibold py-3 rounded-lg transition-colors"
              >
                Register Device
              </button>
            </form>
          </div>
        )}

        {/* Contribute Tab */}
        {activeTab === 'contribute' && (
          <div className="bg-gray-800 rounded-xl p-6 max-w-2xl">
            <h3 className="text-lg font-semibold mb-4">Submit Data Contribution</h3>
            <form onSubmit={handleContribute} className="space-y-4">
              <div>
                <label className="block text-sm text-gray-400 mb-1">Device ID *</label>
                <input
                  type="text"
                  value={contribForm.device_id}
                  onChange={(e) => setContribForm({ ...contribForm, device_id: e.target.value })}
                  placeholder="iot_..."
                  className="w-full bg-gray-700 rounded-lg px-4 py-2 focus:outline-none focus:ring-2 focus:ring-cyan-500"
                  required
                />
              </div>
              
              <div>
                <label className="block text-sm text-gray-400 mb-1">Data Type *</label>
                <select
                  value={contribForm.data_type}
                  onChange={(e) => setContribForm({ ...contribForm, data_type: e.target.value })}
                  className="w-full bg-gray-700 rounded-lg px-4 py-2 focus:outline-none focus:ring-2 focus:ring-cyan-500"
                >
                  <option value="sensor_data">Sensor Data</option>
                  <option value="image">Image</option>
                  <option value="video">Video</option>
                  <option value="audio">Audio</option>
                  <option value="location_trace">Location Trace</option>
                  <option value="sensor_array">Sensor Array</option>
                  <option value="telemetry">Telemetry</option>
                </select>
              </div>
              
              <div>
                <label className="block text-sm text-gray-400 mb-1">Data Hash (optional)</label>
                <input
                  type="text"
                  value={contribForm.data_hash}
                  onChange={(e) => setContribForm({ ...contribForm, data_hash: e.target.value })}
                  placeholder="SHA256 hash of your data (auto-generated if empty)"
                  className="w-full bg-gray-700 rounded-lg px-4 py-2 focus:outline-none focus:ring-2 focus:ring-cyan-500"
                />
              </div>
              
              <div>
                <label className="block text-sm text-gray-400 mb-1">Data Size (bytes) *</label>
                <input
                  type="number"
                  value={contribForm.data_size_bytes}
                  onChange={(e) => setContribForm({ ...contribForm, data_size_bytes: parseInt(e.target.value) || 0 })}
                  min={1}
                  max={1048576}
                  className="w-full bg-gray-700 rounded-lg px-4 py-2 focus:outline-none focus:ring-2 focus:ring-cyan-500"
                  required
                />
                <p className="text-xs text-gray-500 mt-1">Max: 1 MB (1,048,576 bytes)</p>
              </div>
              
              <div className="bg-gray-700 rounded-lg p-4">
                <h4 className="text-sm font-semibold mb-2">Estimated Reward</h4>
                <p className="text-cyan-400 text-lg">
                  ~{(0.1 + (contribForm.data_size_bytes / 1024) * 0.001).toFixed(4)} EDGE
                </p>
                <p className="text-xs text-gray-500 mt-1">
                  Final reward depends on quality score and device multiplier
                </p>
              </div>
              
              <button
                type="submit"
                className="w-full bg-green-600 hover:bg-green-700 text-white font-semibold py-3 rounded-lg transition-colors"
              >
                Submit Data
              </button>
            </form>
          </div>
        )}
      </div>
    </div>
  );
}
