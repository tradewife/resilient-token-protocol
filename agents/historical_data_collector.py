"""
Historical Data Collection Agent for Swarm Trading System
Gathers and manages historical data for backtesting with future-blind simulations
"""

import asyncio
import aiohttp
import pandas as pd
import numpy as np
from datetime import datetime, timedelta
from typing import Dict, List, Optional, Tuple
import json
import os
from dataclasses import dataclass
from concurrent.futures import ThreadPoolExecutor
import ccxt.async_support as ccxt
from redis import Redis
import pickle
import logging

logging.basicConfig(level=logging.INFO)
logger = logging.getLogger(__name__)


@dataclass
class DataWindow:
    """Represents a time window of market data that agents can see"""
    symbol: str
    exchange: str
    start_time: datetime
    end_time: datetime
    current_time: datetime  # Simulation current time
    data: pd.DataFrame
    
    def get_visible_data(self) -> pd.DataFrame:
        """Returns only data up to current simulation time"""
        return self.data[self.data.index <= self.current_time]
    
    def advance_time(self, minutes: int = 1):
        """Advance simulation time by specified minutes"""
        self.current_time += timedelta(minutes=minutes)
        if self.current_time > self.end_time:
            self.current_time = self.end_time
    
    def has_more_data(self) -> bool:
        """Check if there's more future data available"""
        return self.current_time < self.end_time


class HistoricalDataCollector:
    """Agent responsible for collecting and managing historical market data"""
    
    def __init__(self, redis_client: Optional[Redis] = None):
        self.redis = redis_client or Redis(host='localhost', port=6379, db=0)
        self.exchanges = {}
        self.data_cache = {}
        self.executor = ThreadPoolExecutor(max_workers=10)
        
    async def initialize_exchanges(self, exchange_configs: Dict[str, Dict]):
        """Initialize multiple exchange connections"""
        for name, config in exchange_configs.items():
            try:
                if name == 'binance':
                    exchange = ccxt.binance(config)
                elif name == 'coinbase':
                    exchange = ccxt.coinbase(config)
                elif name == 'kraken':
                    exchange = ccxt.kraken(config)
                else:
                    logger.warning(f"Unknown exchange: {name}")
                    continue
                    
                self.exchanges[name] = exchange
                await exchange.load_markets()
                logger.info(f"Initialized {name} exchange")
            except Exception as e:
                logger.error(f"Failed to initialize {name}: {e}")
    
    async def fetch_ohlcv_data(self, exchange_name: str, symbol: str, 
                              timeframe: str = '1m', since: Optional[int] = None,
                              limit: int = 1000) -> pd.DataFrame:
        """Fetch OHLCV data from exchange"""
        try:
            exchange = self.exchanges.get(exchange_name)
            if not exchange:
                raise ValueError(f"Exchange {exchange_name} not initialized")
            
            # Fetch data
            ohlcv = await exchange.fetch_ohlcv(symbol, timeframe, since, limit)
            
            # Convert to DataFrame
            df = pd.DataFrame(ohlcv, columns=['timestamp', 'open', 'high', 'low', 'close', 'volume'])
            df['timestamp'] = pd.to_datetime(df['timestamp'], unit='ms')
            df.set_index('timestamp', inplace=True)
            
            # Cache the data
            cache_key = f"{exchange_name}:{symbol}:{timeframe}"
            self.data_cache[cache_key] = df
            
            # Store in Redis for persistence
            self.redis.setex(
                f"hist_data:{cache_key}",
                86400,  # 24 hour expiry
                pickle.dumps(df)
            )
            
            return df
            
        except Exception as e:
            logger.error(f"Error fetching OHLCV data: {e}")
            raise
    
    async def fetch_multi_timeframe_data(self, exchange_name: str, symbol: str,
                                       timeframes: List[str], days_back: int = 30) -> Dict[str, pd.DataFrame]:
        """Fetch data for multiple timeframes"""
        results = {}
        since = int((datetime.now() - timedelta(days=days_back)).timestamp() * 1000)
        
        tasks = []
        for tf in timeframes:
            task = self.fetch_ohlcv_data(exchange_name, symbol, tf, since)
            tasks.append(task)
        
        dfs = await asyncio.gather(*tasks, return_exceptions=True)
        
        for tf, df in zip(timeframes, dfs):
            if not isinstance(df, Exception):
                results[tf] = df
            else:
                logger.error(f"Failed to fetch {tf} data: {df}")
        
        return results
    
    async def create_data_window(self, exchange_name: str, symbol: str,
                               start_date: datetime, end_date: datetime,
                               timeframe: str = '1m') -> DataWindow:
        """Create a DataWindow for future-blind simulation"""
        # Fetch historical data for the period
        since = int(start_date.timestamp() * 1000)
        until = int(end_date.timestamp() * 1000)
        
        all_data = []
        current_since = since
        
        while current_since < until:
            df = await self.fetch_ohlcv_data(
                exchange_name, symbol, timeframe, 
                since=current_since, limit=1000
            )
            
            if df.empty:
                break
                
            all_data.append(df)
            current_since = int(df.index[-1].timestamp() * 1000) + 1
        
        if not all_data:
            raise ValueError("No data fetched for the specified period")
        
        # Combine all data
        combined_df = pd.concat(all_data)
        combined_df = combined_df[~combined_df.index.duplicated(keep='first')]
        combined_df.sort_index(inplace=True)
        
        # Filter to exact date range
        mask = (combined_df.index >= start_date) & (combined_df.index <= end_date)
        filtered_df = combined_df.loc[mask]
        
        return DataWindow(
            symbol=symbol,
            exchange=exchange_name,
            start_time=start_date,
            end_time=end_date,
            current_time=start_date,
            data=filtered_df
        )
    
    async def fetch_order_book_snapshot(self, exchange_name: str, symbol: str) -> Dict:
        """Fetch current order book snapshot"""
        try:
            exchange = self.exchanges.get(exchange_name)
            if not exchange:
                raise ValueError(f"Exchange {exchange_name} not initialized")
            
            order_book = await exchange.fetch_order_book(symbol)
            return {
                'timestamp': datetime.now(),
                'bids': order_book['bids'][:20],  # Top 20 bids
                'asks': order_book['asks'][:20],  # Top 20 asks
                'symbol': symbol,
                'exchange': exchange_name
            }
        except Exception as e:
            logger.error(f"Error fetching order book: {e}")
            return {}
    
    async def fetch_recent_trades(self, exchange_name: str, symbol: str, limit: int = 100) -> List[Dict]:
        """Fetch recent trades"""
        try:
            exchange = self.exchanges.get(exchange_name)
            if not exchange:
                raise ValueError(f"Exchange {exchange_name} not initialized")
            
            trades = await exchange.fetch_trades(symbol, limit=limit)
            return [{
                'timestamp': trade['timestamp'],
                'price': trade['price'],
                'amount': trade['amount'],
                'side': trade['side'],
                'id': trade['id']
            } for trade in trades]
        except Exception as e:
            logger.error(f"Error fetching trades: {e}")
            return []
    
    def calculate_technical_indicators(self, df: pd.DataFrame) -> pd.DataFrame:
        """Calculate technical indicators on historical data"""
        # Simple Moving Averages
        df['sma_20'] = df['close'].rolling(window=20).mean()
        df['sma_50'] = df['close'].rolling(window=50).mean()
        
        # Exponential Moving Averages
        df['ema_12'] = df['close'].ewm(span=12, adjust=False).mean()
        df['ema_26'] = df['close'].ewm(span=26, adjust=False).mean()
        
        # MACD
        df['macd'] = df['ema_12'] - df['ema_26']
        df['macd_signal'] = df['macd'].ewm(span=9, adjust=False).mean()
        df['macd_diff'] = df['macd'] - df['macd_signal']
        
        # RSI
        delta = df['close'].diff()
        gain = (delta.where(delta > 0, 0)).rolling(window=14).mean()
        loss = (-delta.where(delta < 0, 0)).rolling(window=14).mean()
        rs = gain / loss
        df['rsi'] = 100 - (100 / (1 + rs))
        
        # Bollinger Bands
        df['bb_middle'] = df['close'].rolling(window=20).mean()
        bb_std = df['close'].rolling(window=20).std()
        df['bb_upper'] = df['bb_middle'] + (bb_std * 2)
        df['bb_lower'] = df['bb_middle'] - (bb_std * 2)
        
        # Volume indicators
        df['volume_sma'] = df['volume'].rolling(window=20).mean()
        df['volume_ratio'] = df['volume'] / df['volume_sma']
        
        return df
    
    async def close_all(self):
        """Close all exchange connections"""
        for exchange in self.exchanges.values():
            await exchange.close()
        self.executor.shutdown()


class HistoricalDataSwarm:
    """Manages a swarm of data collection agents"""
    
    def __init__(self, num_agents: int = 5):
        self.agents = []
        self.num_agents = num_agents
        self.redis = Redis(host='localhost', port=6379, db=0)
        
    async def initialize(self, exchange_configs: Dict[str, Dict]):
        """Initialize the swarm of data collectors"""
        for i in range(self.num_agents):
            agent = HistoricalDataCollector(self.redis)
            await agent.initialize_exchanges(exchange_configs)
            self.agents.append(agent)
            logger.info(f"Initialized data collector agent {i}")
    
    async def collect_parallel(self, tasks: List[Tuple[str, str, str]]) -> Dict:
        """Distribute data collection tasks across agents"""
        results = {}
        
        # Distribute tasks among agents
        agent_tasks = [[] for _ in range(self.num_agents)]
        for i, task in enumerate(tasks):
            agent_idx = i % self.num_agents
            agent_tasks[agent_idx].append(task)
        
        # Execute tasks in parallel
        async def process_agent_tasks(agent_idx: int, tasks: List[Tuple]):
            agent = self.agents[agent_idx]
            agent_results = {}
            
            for exchange, symbol, timeframe in tasks:
                try:
                    df = await agent.fetch_ohlcv_data(exchange, symbol, timeframe)
                    agent_results[f"{exchange}:{symbol}:{timeframe}"] = df
                except Exception as e:
                    logger.error(f"Agent {agent_idx} failed task {exchange}:{symbol}:{timeframe}: {e}")
            
            return agent_results
        
        # Run all agents in parallel
        agent_results = await asyncio.gather(*[
            process_agent_tasks(i, tasks) 
            for i, tasks in enumerate(agent_tasks)
        ])
        
        # Combine results
        for result in agent_results:
            results.update(result)
        
        return results
    
    async def close_all(self):
        """Close all agents"""
        for agent in self.agents:
            await agent.close_all()