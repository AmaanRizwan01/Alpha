"""YouTube Strategy Rule Extractor (Dual-Mode: CLI + Agent).

Pulls video transcripts using youtube-transcript-api (no API key required)
and parses them into structured trading rules.
"""

import os
import re
from typing import Optional
from pathlib import Path


def extract_video_id(url: str) -> Optional[str]:
    """Extract YouTube video ID from various URL formats."""
    patterns = [
        r"(?:v=|\/)([0-9A-Za-z_-]{11}).*",
        r"(?:youtu\.be\/)([0-9A-Za-z_-]{11})",
        r"(?:embed\/)([0-9A-Za-z_-]{11})",
        r"(?:shorts\/)([0-9A-Za-z_-]{11})",
    ]
    for pattern in patterns:
        match = re.search(pattern, url)
        if match:
            return match.group(1)
    return None


def fetch_youtube_transcript(url_or_id: str) -> str:
    """Fetch transcript text from YouTube video without an API key."""
    video_id = extract_video_id(url_or_id) or url_or_id
    try:
        from youtube_transcript_api import YouTubeTranscriptApi
        
        # In youtube-transcript-api v0.6+, get_transcript returns list of dicts with 'text'
        transcript_data = YouTubeTranscriptApi.get_transcript(video_id)
        full_text = " ".join(item.get("text", "") for item in transcript_data)
        return full_text
    except ImportError:
        raise ImportError(
            "youtube-transcript-api is not installed. Run: pip install youtube-transcript-api"
        )
    except Exception as exc:
        raise RuntimeError(f"Failed to fetch YouTube transcript: {exc}")


STRATEGY_EXTRACTION_PROMPT = """
You are a Lead Quantitative Research Analyst. Analyze this YouTube trading tutorial transcript
and extract the exact deterministic rules into structured JSON.

Transcript:
\"\"\"
{transcript}
\"\"\"

Output valid JSON matching this exact structure:
{{
  "strategy_name": "<short_clean_name>",
  "target_asset": "BTC/USDT",
  "timeframes": {{
    "higher_timeframe": "4h or 1d (for trend/bias)",
    "execution_timeframe": "15m or 1h (for entry trigger)"
  }},
  "bias_rules": [
    "e.g. price > 200 EMA or liquidity swept"
  ],
  "entry_trigger": [
    "e.g. Fair Value Gap retest, RSI < 30, or Breakout"
  ],
  "stop_loss": {{
    "type": "structural_swing or atr_multiple",
    "rule": "e.g. 2.0 * ATR14 below entry"
  }},
  "take_profit": {{
    "type": "r_multiple or trailing_stop",
    "rule": "e.g. 2.5R or trail 20 SMA"
  }},
  "max_hold_bars": 40
}}
"""


def extract_and_save(url: str, name: Optional[str] = None) -> Path:
    """Extract transcript and save draft specification."""
    video_id = extract_video_id(url)
    if not video_id:
        raise ValueError(f"Could not parse valid YouTube video ID from: {url}")
    
    slug = name or f"yt_{video_id}"
    transcript = fetch_youtube_transcript(video_id)
    
    drafts_dir = Path("python/research/strategies/drafts") / slug
    drafts_dir.mkdir(parents=True, exist_ok=True)
    
    transcript_file = drafts_dir / "transcript.txt"
    transcript_file.write_text(transcript, encoding="utf-8")
    
    prompt_file = drafts_dir / "extraction_prompt.txt"
    prompt_file.write_text(
        STRATEGY_EXTRACTION_PROMPT.format(transcript=transcript[:12000]),
        encoding="utf-8"
    )
    
    return drafts_dir
