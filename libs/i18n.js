/**
 * NASA Breakup Model Visualizer - Decoupled Localization Catalog (i18n)
 * Supports English (en) and Japanese (ja)
 */
(function (global) {
  'use strict';
  const window = global;

  const translations = {
    en: {
      // Header
      'header.title': 'NASA Breakup Model Visualizer',
      'header.badge': 'Built-in Simple Engine',
      'header.subtitle': '0–15 minute satellite collision, fragment yield & orbital dispersion',
      'header.btn_gabbard': '📊 Gabbard Diagram',
      'header.btn_volumes': '🌐 Hazard Volumes',
      'header.btn_inspector': '🎯 Inspector & Trails',

      // Section 1: Collision Setup
      'sec1.title': '1. Collision Setup',
      'sec1.target_mass': 'Target Mass (M₁)',
      'sec1.proj_mass': 'Projectile Mass (M₂)',
      'sec1.impact_speed': 'Impact Speed (v_imp)',
      'sec1.target_class': 'Target Object Class',
      'sec1.class_sc': 'Spacecraft (S/C)',
      'sec1.class_rb': 'Rocket Body (R/B)',
      'sec1.regime': 'Breakup Event Regime',
      'sec1.regime_collision': 'Collision',
      'sec1.regime_explosion': 'Explosion',
      'sec1.explosion_scale': 'Explosion Scaling (S)',
      'sec1.explosion_hint': 'S=1.0 nominal upper stage (Johnson 2001, Eq. 3)',
      'sec1.shapes': 'Object Shapes',
      'sec1.shape_spheres': 'Spheres',
      'sec1.shape_boxes': 'Satellite Boxes',

      // Section 2: Simple Engine Rules
      'sec2.title': '2. Simple Engine Rules',
      'sec2.resolution': 'Fragment Resolution',
      'sec2.skew': 'Size Skew (Power-Law)',
      'sec2.skew_hint': 'Higher = far more tiny shards than large chunks',
      'sec2.speedbias': 'Light-to-Heavy Speed Bias',
      'sec2.speedbias_hint': 'Higher = light pieces fly significantly faster',
      'sec2.blast_pattern': 'Blast Direction Pattern',
      'sec2.blast_spherical': 'Spherical Burst',
      'sec2.blast_cone': 'Forward Impact Cone',

      // Section 3: Speed Contour & Density Analytics
      'sec3.title': '3. Speed Contour & Density Analytics',
      'sec3.btn_contours': 'Speed Contours',
      'sec3.btn_density': 'Density Heatmap (ρ)',
      'sec3.btn_smooth': 'Smooth Gradient',
      'sec3.btn_origin': 'Parent Origin',
      'sec3.check_volumes': 'Show 1σ / 2σ Hazard Shells',
      'sec3.btn_reset_cam': 'Reset Cam',
      'sec3.legend_speed_title': 'Speed Iso-Contours (Δv)',
      'sec3.legend_density_title': 'Volumetric Debris Density (ρ)',
      'sec3.legend_density_unit': 'rel. to peak',
      'sec3.legend_density_label1': 'Dispersed 2σ',
      'sec3.legend_density_label2': 'Mid Envelope',
      'sec3.legend_density_label3': '1σ Hazard Core',
      'sec3.legend_origin_title': 'Fragment Parent Origin',
      'sec3.legend_origin_unit': 'satellite',
      'sec3.legend_origin_target': 'Target Satellite (M₁)',
      'sec3.legend_origin_impactor': 'Impactor / Interceptor (M₂)',
      'sec3.legend_origin_label1': 'Target Body',
      'sec3.legend_origin_label2': 'Impactor Body',
      'sec3.legend_smooth_title': 'Continuous Velocity Spectrum',
      'sec3.legend_smooth_unit': '0.1 – 2.2 km/s',
      'sec3.legend_smooth_label1': 'Low Δv (<300 m/s)',
      'sec3.legend_smooth_label2': 'Median (~1.1 km/s)',
      'sec3.legend_smooth_label3': 'Ejecta (>2.2 km/s)',
      'sec3.legend_slow_core': 'Slow Core',
      'sec3.legend_mid_shell': 'Mid Shell',
      'sec3.legend_hyper_bubble': 'Hypersonic Bubble',
      'sec3.weight_scaling_title': 'Weight Ratio Scaling:',
      'sec3.weight_scaling_desc': 'Linear Size R ∝ ∛Mass (Physical Volume Ratio)',

      // Section 4: Orbit Trails & Tracking
      'sec4.title': '4. Orbit Trails & Tracking',
      'sec4.trail_mode': 'Orbit Trail Mode',
      'sec4.trail_off': 'Off',
      'sec4.trail_selected': 'Selected',
      'sec4.trail_heaviest': 'Top 5 Heavy',
      'sec4.trail_fastest': 'Top 5 Fast',
      'sec4.trail_cardinal': '4 Cardinal',
      'sec4.cam_tracking': 'Camera Tracking',
      'sec4.cam_free': 'Free Orbit',
      'sec4.cam_follow': 'Follow Selected',
      'sec4.point_scale': 'Point Size Scale',

      // Right Sidebar: Live Engine Telemetry
      'telemetry.title': 'Live Engine Telemetry',
      'telemetry.impact_energy': 'Impact Energy',
      'telemetry.outcome': 'Outcome',
      'telemetry.catastrophic': 'Catastrophic',
      'telemetry.partial': 'Partial/Crater',
      'telemetry.explosion': 'Explosion (S={s})',
      'telemetry.destroyed_mass': 'Destroyed Mass',
      'telemetry.remnant': '({remnant}kg remnant)',
      'telemetry.max_dv': 'Max Δv Kick',
      'telemetry.cloud_width': 'Cloud Width (Y)',
      'telemetry.stage': 'Stage',
      'telemetry.not_applicable_explosion': 'N/A (Explosion)',
      'telemetry.stage_approaching': 'Approaching ({s}s to impact)',
      'telemetry.stage_impact': '💥 IMPACT CONTACT',
      'telemetry.stage_collision_core': 'Collision Core (∞)',
      'telemetry.stage_spherical': 'Spherical Blast (0–2m)',
      'telemetry.stage_shear': 'Keplerian Shear (2–15m)',
      'telemetry.stage_secular': 'Secular Cloud (>15m)',
      'telemetry.peak_density': 'Peak Density (ρ)',
      'telemetry.reentry_risk': 'Re-entry Risk',
      'telemetry.phys_yield': 'Physical Yield (≥1cm)',
      'telemetry.ssn_trackable': 'SSN Trackable (≥10cm)',
      'telemetry.table_title': 'Fragment Yield by Weight & Speed',
      'telemetry.row_tooltip': 'Click to inspect Fragment #{id} & show its 3D orbit trail',
      'telemetry.th_class': 'Class',
      'telemetry.th_weight': 'Weight',
      'telemetry.th_size_ratio': 'Size Ratio',
      'telemetry.th_speed': 'Speed',
      'telemetry.th_contour': 'Contour',
      'telemetry.class_heavy': 'Heavy Core',
      'telemetry.class_medium': 'Medium Shell',
      'telemetry.class_light': 'Light Shard',
      'telemetry.color_blue': '<250 (Blue)',
      'telemetry.color_cyan': '250-500 (Cyan)',
      'telemetry.color_green': '500-1k (Green)',
      'telemetry.color_yellow': '1k-1.5k (Yellow)',
      'telemetry.color_orange': '1.5k-2k (Orange)',
      'telemetry.color_red': '>2k (Red)',
      'telemetry.legend_note_heavy': 'Heavy core: high weight ratio, slow & clustered.',
      'telemetry.legend_note_light': 'Light shards: low weight ratio, fast outer contour shell.',

      // Floating Fragment Inspector HUD
      'inspector.title': 'Fragment Inspector',
      'inspector.mass': 'Mass',
      'inspector.lc': 'Length (Lc)',
      'inspector.area': 'Area (Ax)',
      'inspector.am': 'A/M Ratio',
      'inspector.bstar': 'B* Drag Coeff',
      'inspector.speed': 'Ejection Δv',
      'inspector.ha': 'Apogee (ha)',
      'inspector.hp': 'Perigee (hp)',
      'inspector.period': 'Period (P)',
      'inspector.status': 'Status',
      'inspector.parent': 'Parent:',
      'inspector.class': 'Class:',
      'inspector.status_impact': '💥 Surface Impact',
      'inspector.status_reentry': '⚠️ Re-entry Decay',
      'inspector.status_stable': '✓ Stable Orbit',
      'inspector.parent_target': 'Target Satellite',
      'inspector.parent_projectile': 'Impactor Projectile',
      'inspector.class_spacecraft': 'Spacecraft',
      'inspector.class_rocket_body': 'Rocket Body',
      'inspector.btn_follow_title': 'Toggle Camera Tracking',
      'inspector.btn_close_title': 'Close / Deselect',

      // Floating Gabbard Diagram
      'gabbard.title': 'Gabbard Diagram',
      'gabbard.badge': 'Orbital Period vs Altitude',
      'gabbard.ref': 'Ref: h₀ = 500 km (T₀ = 94.6m)',
      'gabbard.max_ha': 'Max Apogee:',
      'gabbard.reentry_risk': 'Re-entry Risk:',
      'gabbard.apogee': 'Apogee (ha)',
      'gabbard.perigee': 'Perigee (hp)',
      'gabbard.reentry_threshold': 'Re-entry <120km',
      'gabbard.hover_hint': 'Hover point to inspect & highlight in 3D',
      'gabbard.min_title': 'Minimize / Restore',
      'gabbard.close_title': 'Close',
      'gabbard.canvas_loading': 'Simulating orbital elements...',
      'gabbard.canvas_reentry': 'Re-entry Interface (120 km)',
      'gabbard.canvas_breakup_ref': 'h₀=500km, T₀=94.6m',
      'gabbard.canvas_x_label': 'Orbital Period P (minutes)',
      'gabbard.canvas_y_label': 'Altitude h (km)',
      'gabbard.tt_leo_outlier': '(LEO Outlier)',
      'gabbard.tt_ground_impact': '(Ground Impact)',
      'gabbard.tt_reentry_risk': '⚠️ Re-entry Risk (<120 km)',
      'gabbard.tt_stable_orbit': '✓ Stable Orbit',

      // Bottom Timeline Bar
      'timeline.mode_label': 'TIMELINE MODE:',
      'timeline.mode_zoom': 'Impact Zoom (-10s to +30s)',
      'timeline.mode_full': 'Full 15-Min Orbit',
      'timeline.collision_pin': 'Collision Pin: T = 0.0s [Impact]',
      'timeline.time_impact': 'T 00.0s [IMPACT]',
      'timeline.mark_impact': 'T=0s [IMPACT]'
    },

    ja: {
      // Header
      'header.title': 'NASA破砕モデル可視化ツール',
      'header.badge': '簡易エンジン内蔵',
      'header.subtitle': '0〜15分間の衛星衝突・破片生成および軌道分散シミュレーション',
      'header.btn_gabbard': '📊 ガバード図',
      'header.btn_volumes': '🌐 危険領域',
      'header.btn_inspector': '🎯 軌道追跡・詳細検査',

      // Section 1: Collision Setup
      'sec1.title': '1. 衝突条件設定',
      'sec1.target_mass': '標的質量 (M₁)',
      'sec1.proj_mass': '衝突体質量 (M₂)',
      'sec1.impact_speed': '衝突速度 (v_imp)',
      'sec1.target_class': '標的オブジェクト種別',
      'sec1.class_sc': '人工衛星 (S/C)',
      'sec1.class_rb': 'ロケット上段 (R/B)',
      'sec1.regime': '破砕イベント区分',
      'sec1.regime_collision': '超高速衝突',
      'sec1.regime_explosion': '爆発',
      'sec1.explosion_scale': '爆発スケーリング係数 (S)',
      'sec1.explosion_hint': 'S=1.0: 標準的ロケット上段 (Johnson 2001, 式3)',
      'sec1.shapes': 'オブジェクト形状',
      'sec1.shape_spheres': '球体',
      'sec1.shape_boxes': '衛星モデル',

      // Section 2: Simple Engine Rules
      'sec2.title': '2. 物理エンジン設定',
      'sec2.resolution': '破片生成解像度',
      'sec2.skew': 'サイズ偏向度 (冪乗則)',
      'sec2.skew_hint': '高い値 = 微小破片が急増、大破片が減少',
      'sec2.speedbias': '質量-速度バイアス',
      'sec2.speedbias_hint': '高い値 = 軽量破片がより高速に飛散',
      'sec2.blast_pattern': '飛散方向パターン',
      'sec2.blast_spherical': '等方性球状飛散',
      'sec2.blast_cone': '衝突前方コーン',

      // Section 3: Speed Contour & Density Analytics
      'sec3.title': '3. 速度等高線・密度解析',
      'sec3.btn_contours': '速度コンター',
      'sec3.btn_density': '空間密度マップ (ρ)',
      'sec3.btn_smooth': 'スムーズ階調',
      'sec3.btn_origin': '発生源別',
      'sec3.check_volumes': '1σ / 2σ 危険エンベロープを表示',
      'sec3.btn_reset_cam': '視点リセット',
      'sec3.legend_speed_title': '速度等高線 (Δv)',
      'sec3.legend_density_title': '空間破片密度 (ρ)',
      'sec3.legend_density_unit': '最大比',
      'sec3.legend_density_label1': '拡散層 2σ',
      'sec3.legend_density_label2': '中間エンベロープ',
      'sec3.legend_density_label3': '1σ 危険コア',
      'sec3.legend_origin_title': '破片発生源',
      'sec3.legend_origin_unit': '衛星種別',
      'sec3.legend_origin_target': '標的衛星 (M₁)',
      'sec3.legend_origin_impactor': '衝突体 (M₂)',
      'sec3.legend_origin_label1': '標的側',
      'sec3.legend_origin_label2': '衝突体側',
      'sec3.legend_smooth_title': '速度連続スペクトラム',
      'sec3.legend_smooth_unit': '0.1〜2.2 km/s',
      'sec3.legend_smooth_label1': '低速Δv (<300 m/s)',
      'sec3.legend_smooth_label2': '中央値 (~1.1 km/s)',
      'sec3.legend_smooth_label3': '高速飛散 (>2.2 km/s)',
      'sec3.legend_slow_core': '低速コア',
      'sec3.legend_mid_shell': '中速シェル',
      'sec3.legend_hyper_bubble': '超高速シェル',
      'sec3.weight_scaling_title': '質量比スケーリング:',
      'sec3.weight_scaling_desc': '表示半径 R ∝ ∛質量 (体積比率準拠)',

      // Section 4: Orbit Trails & Tracking
      'sec4.title': '4. 軌道トレール & 追跡',
      'sec4.trail_mode': '軌道トレール表示',
      'sec4.trail_off': 'オフ',
      'sec4.trail_selected': '選択破片',
      'sec4.trail_heaviest': '質量上位5件',
      'sec4.trail_fastest': '速度上位5件',
      'sec4.trail_cardinal': '主要4軸',
      'sec4.cam_tracking': 'カメラ追跡',
      'sec4.cam_free': '自由視点',
      'sec4.cam_follow': '選択破片追従',
      'sec4.point_scale': 'ポイント表示倍率',

      // Right Sidebar: Live Engine Telemetry
      'telemetry.title': 'リアルタイム・テレメトリ',
      'telemetry.impact_energy': '比衝突エネルギー',
      'telemetry.outcome': '衝突結果',
      'telemetry.catastrophic': '破局的破壊',
      'telemetry.partial': '非破局的 (クレーター)',
      'telemetry.explosion': '爆発 (S={s})',
      'telemetry.destroyed_mass': '破壊質量',
      'telemetry.remnant': '(残存質量: {remnant}kg)',
      'telemetry.max_dv': '最大Δvキック',
      'telemetry.cloud_width': '雲幅 (進行方向 Y)',
      'telemetry.stage': '進行フェーズ',
      'telemetry.not_applicable_explosion': '対象外 (爆発)',
      'telemetry.stage_approaching': '接近中 (衝突まで {s}秒)',
      'telemetry.stage_impact': '💥 衝突点 (インパクト)',
      'telemetry.stage_collision_core': '衝突直後コア (∞)',
      'telemetry.stage_spherical': '初期球状飛散 (0〜2分)',
      'telemetry.stage_shear': 'ケプラー剪断 (2〜15分)',
      'telemetry.stage_secular': '長期的軌道雲 (>15分)',
      'telemetry.peak_density': '最大空間密度 (ρ)',
      'telemetry.reentry_risk': '大気圏再突入リスク',
      'telemetry.phys_yield': '推定破片総数 (≥1cm)',
      'telemetry.ssn_trackable': 'SSN追跡可能数 (≥10cm)',
      'telemetry.table_title': '質量・速度別破片サンプル',
      'telemetry.row_tooltip': 'クリックで破片 #{id} を詳細検査 & 3D軌道トレール表示',
      'telemetry.th_class': '区分',
      'telemetry.th_weight': '質量',
      'telemetry.th_size_ratio': 'サイズ比',
      'telemetry.th_speed': '速度',
      'telemetry.th_contour': '色',
      'telemetry.class_heavy': '重量コア',
      'telemetry.class_medium': '中量シェル',
      'telemetry.class_light': '軽量破片',
      'telemetry.color_blue': '<250 (青)',
      'telemetry.color_cyan': '250-500 (水色)',
      'telemetry.color_green': '500-1k (緑)',
      'telemetry.color_yellow': '1k-1.5k (黄)',
      'telemetry.color_orange': '1.5k-2k (橙)',
      'telemetry.color_red': '>2k (赤)',
      'telemetry.legend_note_heavy': '重量コア: 高質量比、低速で中心付近に集中。',
      'telemetry.legend_note_light': '軽量破片: 低質量比、高速で外郭シェルへ拡散。',

      // Floating Fragment Inspector HUD
      'inspector.title': '破片インスペクター',
      'inspector.mass': '質量',
      'inspector.lc': '特性長 (Lc)',
      'inspector.area': '断面積 (Ax)',
      'inspector.am': '断面積質量比 (A/M)',
      'inspector.bstar': 'B* 弾道減衰係数',
      'inspector.speed': '放出速度 (Δv)',
      'inspector.ha': '遠地点高度 (ha)',
      'inspector.hp': '近地点高度 (hp)',
      'inspector.period': '軌道周期 (P)',
      'inspector.status': '軌道状態',
      'inspector.parent': '発生元:',
      'inspector.class': '種別:',
      'inspector.status_impact': '💥 地表衝突',
      'inspector.status_reentry': '⚠️ 再突入減衰',
      'inspector.status_stable': '✓ 安定軌道',
      'inspector.parent_target': '標的衛星',
      'inspector.parent_projectile': '衝突体',
      'inspector.class_spacecraft': '人工衛星',
      'inspector.class_rocket_body': 'ロケット上段',
      'inspector.btn_follow_title': 'カメラ追従切替',
      'inspector.btn_close_title': '閉じる / 選択解除',

      // Floating Gabbard Diagram
      'gabbard.title': 'ガバード図',
      'gabbard.badge': '軌道周期 vs 高度',
      'gabbard.ref': '基準: h₀ = 500 km (T₀ = 94.6分)',
      'gabbard.max_ha': '最大遠地点:',
      'gabbard.reentry_risk': '再突入リスク:',
      'gabbard.apogee': '遠地点 (ha)',
      'gabbard.perigee': '近地点 (hp)',
      'gabbard.reentry_threshold': '再突入境界 <120km',
      'gabbard.hover_hint': 'プロットにホバーで3D強調表示',
      'gabbard.min_title': '最小化 / 元に戻す',
      'gabbard.close_title': '閉じる',
      'gabbard.canvas_loading': '軌道要素を計算中...',
      'gabbard.canvas_reentry': '大気圏再突入高度 (120 km)',
      'gabbard.canvas_breakup_ref': 'h₀=500km, T₀=94.6分',
      'gabbard.canvas_x_label': '軌道周期 P (分)',
      'gabbard.canvas_y_label': '高度 h (km)',
      'gabbard.tt_leo_outlier': '(LEO逸脱)',
      'gabbard.tt_ground_impact': '(地表衝突)',
      'gabbard.tt_reentry_risk': '⚠️ 再突入リスク (<120 km)',
      'gabbard.tt_stable_orbit': '✓ 安定軌道',

      // Bottom Timeline Bar
      'timeline.mode_label': 'タイムライン表示:',
      'timeline.mode_zoom': '衝突ズーム (-10s〜+30s)',
      'timeline.mode_full': '全15分軌道 (-10s〜+900s)',
      'timeline.collision_pin': '衝突点: T = 0.0s [衝突]',
      'timeline.time_impact': 'T 00.0s [衝突]',
      'timeline.mark_impact': 'T=0s [衝突]'
    }
  };

  const I18N = {
    currentLang: 'en',
    translations: translations,

    t(key, params = {}) {
      const dict = translations[this.currentLang] || translations.en;
      let text = dict[key] || translations.en[key] || key;
      for (const [k, v] of Object.entries(params)) {
        text = text.replace(new RegExp(`\\{${k}\\}`, 'g'), v);
      }
      return text;
    },

    setLanguage(lang) {
      if (!['en', 'ja'].includes(lang)) lang = 'en';
      this.currentLang = lang;
      try {
        localStorage.setItem('sbm_lang', lang);
      } catch (e) {}

      if (typeof document !== 'undefined') {
        if (document.documentElement) document.documentElement.lang = lang;

        // Update static data-i18n elements
        document.querySelectorAll('[data-i18n]').forEach((el) => {
          const key = el.getAttribute('data-i18n');
          el.textContent = this.t(key);
        });

        // Update data-i18n-html elements
        document.querySelectorAll('[data-i18n-html]').forEach((el) => {
          const key = el.getAttribute('data-i18n-html');
          el.innerHTML = this.t(key);
        });

        // Update data-i18n-title elements
        document.querySelectorAll('[data-i18n-title]').forEach((el) => {
          const key = el.getAttribute('data-i18n-title');
          el.title = this.t(key);
        });

        // Update selector buttons
        const btnEn = document.getElementById('lang-en');
        const btnJa = document.getElementById('lang-ja');
        if (btnEn) btnEn.classList.toggle('active', lang === 'en');
        if (btnJa) btnJa.classList.toggle('active', lang === 'ja');
      }

      // Dispatch event for components that need dynamic rerender
      if (typeof window !== 'undefined' && window.dispatchEvent && typeof CustomEvent !== 'undefined') {
        window.dispatchEvent(new CustomEvent('languageChanged', { detail: { lang } }));
      }
    },

    init() {
      let lang = 'en';
      try {
        const searchStr = window.location.search || (window.location.href.includes('?') ? window.location.href.slice(window.location.href.indexOf('?')) : '');
        const urlParams = new URLSearchParams(searchStr);
        if (urlParams.has('lang')) {
          const l = urlParams.get('lang').toLowerCase();
          if (['en', 'ja'].includes(l)) lang = l;
        } else {
          const stored = localStorage.getItem('sbm_lang');
          if (stored && ['en', 'ja'].includes(stored)) {
            lang = stored;
          } else if (navigator.language && navigator.language.startsWith('ja')) {
            lang = 'ja';
          }
        }
      } catch (e) {}
      this.currentLang = lang;
    }
  };

  I18N.init();
  global.I18N = I18N;
  if (typeof module !== 'undefined' && module.exports) {
    module.exports = I18N;
  }
})(typeof window !== 'undefined' ? window : globalThis);
