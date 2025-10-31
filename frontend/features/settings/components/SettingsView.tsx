import { Avatar, AvatarFallback } from '@/components/ui/avatar';
import { Button } from '@/components/ui/button';
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuTrigger,
} from '@/components/ui/dropdown-menu';
import { Label } from '@/components/ui/label';
import { ScrollArea } from '@/components/ui/scroll-area';
import { Separator } from '@/components/ui/separator';
import { Slider } from '@/components/ui/slider';
import { Switch } from '@/components/ui/switch';
import { Tabs, TabsContent, TabsList, TabsTrigger } from '@/components/ui/tabs';
import type { CalendarConnectionStatus } from '@/shared/types/generated/CalendarConnectionStatus';
import type { UserProfile } from '@/shared/types/generated/UserProfile';
import { formatTime, invalidateTimeFormatCache } from '@/shared/utils/timeFormat';
import { invoke } from '@tauri-apps/api/core';
import {
  Activity,
  ArrowLeft,
  Brain,
  Camera,
  Check,
  ChevronDown,
  Clock,
  GripHorizontal,
  HelpCircle,
  LogIn,
  Plug,
  RefreshCw,
  Settings,
  User,
  X,
} from 'lucide-react';
import React from 'react';
import { adminService } from '../services/adminService';
import { calendarService } from '../services/calendarService';
import { SapService } from '../services/sapService';
import { settingsService } from '../services/settingsService';
import { WebApiService } from '../services/WebApiService';
import type { SettingsViewProps } from '../types';
import { CalendarProviderCard } from './CalendarProviderCard';
import { IdleDetectionSettings } from './IdleDetectionSettings';

export function SettingsView({ onBack, onRestartTutorial }: SettingsViewProps) {
  // Load settings synchronously to avoid race condition with save effect
  const initialSettings = React.useMemo(() => settingsService.loadSettings(), []);

  // State declarations - must come before useEffect that uses them
  const [autoApply, setAutoApply] = React.useState(initialSettings.autoApply);
  const [notifications, setNotifications] = React.useState(initialSettings.notifications);
  const [confidence, setConfidence] = React.useState([initialSettings.confidence]);
  const [timeFormat, setTimeFormat] = React.useState<'12h' | '24h'>(initialSettings.timeFormat);
  const [currentTime, setCurrentTime] = React.useState(new Date());

  // Dynamically resize window based on expanded sections (like MainTimer does)
  React.useEffect(() => {
    const resizeWindow = async () => {
      try {
        const { getCurrentWindow } = await import('@tauri-apps/api/window');
        const { LogicalSize } = await import('@tauri-apps/api/window');
        const currentWindow = getCurrentWindow();

        const targetWidth = 580;
        // Base height: 450px
        // Auto-apply suggestions expansion adds ~130px (slider + notifications toggle)
        const baseHeight = 450;
        const autoApplyHeight = 130;

        let targetHeight = baseHeight;
        if (autoApply) targetHeight += autoApplyHeight;

        console.log('[SettingsView] Resizing window:', {
          targetWidth,
          targetHeight,
          autoApply,
          expansions: {
            autoApply: autoApply ? autoApplyHeight : 0,
          },
        });

        await currentWindow.setSize(new LogicalSize(targetWidth, targetHeight));
        await currentWindow.setResizable(false);

        // Lock the size for settings view
        await currentWindow.setMinSize(new LogicalSize(targetWidth, targetHeight));
        await currentWindow.setMaxSize(new LogicalSize(targetWidth, targetHeight));
      } catch (error) {
        console.error('[SettingsView] Failed to resize window:', error);
      }
    };

    void resizeWindow();
  }, [autoApply]); // Re-run when toggle changes
  const integrations = settingsService.getAvailableIntegrations();
  const [connectedIntegrations, setConnectedIntegrations] = React.useState<Record<string, boolean>>(
    initialSettings.integrations
  );
  const [calendarEmail, setCalendarEmail] = React.useState<string | null>(null);
  const [isLoadingGoogle, setIsLoadingGoogle] = React.useState(false);
  const [isLoadingMicrosoft, setIsLoadingMicrosoft] = React.useState(false);
  const [isLoadingSap, setIsLoadingSap] = React.useState(false);
  const [isSyncingCalendar, setIsSyncingCalendar] = React.useState(false);
  const [calendarStatuses, setCalendarStatuses] = React.useState<CalendarConnectionStatus[]>([]);
  const [isClearingData, setIsClearingData] = React.useState(false);
  const [showClearConfirm, setShowClearConfirm] = React.useState(false);
  // FEATURE-016: Main API authentication state
  const [isSignedIn, setIsSignedIn] = React.useState(false);
  const [isSigningIn, setIsSigningIn] = React.useState(false);
  const [_userEmail, setUserEmail] = React.useState<string | null>(null);
  // FEATURE-029 Phase 5: Fetch user profile from database
  const [userProfile, setUserProfile] = React.useState<UserProfile | null>(null);
  const [avatarImage, setAvatarImage] = React.useState<string | null>(null);
  const fileInputRef = React.useRef<HTMLInputElement>(null);

  // Fetch user profile from database on mount
  React.useEffect(() => {
    void (async () => {
      try {
        const profile = await invoke<UserProfile | null>('get_user_profile');
        if (profile) {
          setUserProfile(profile);
          if (profile.avatar_url) {
            setAvatarImage(profile.avatar_url);
          }
        }
      } catch (error) {
        console.error('[SettingsView] Failed to fetch user profile:', error);
      }
    })();
  }, []);

  const handlePhotoUpload = () => {
    fileInputRef.current?.click();
  };

  const handleRemovePhoto = () => {
    setAvatarImage(null);
    if (fileInputRef.current) {
      fileInputRef.current.value = '';
    }
    console.log('[SettingsView] Avatar image removed');
  };

  const handleFileChange = (event: React.ChangeEvent<HTMLInputElement>) => {
    const file = event.target.files?.[0];
    if (file) {
      // Validate file type
      if (!file.type.startsWith('image/')) {
        console.error('[SettingsView] Invalid file type. Please select an image.');
        return;
      }

      // Validate file size (max 5MB)
      if (file.size > 5 * 1024 * 1024) {
        console.error('[SettingsView] File too large. Maximum size is 5MB.');
        return;
      }

      // Read and display the image
      const reader = new FileReader();
      reader.onload = (e) => {
        const imageData = e.target?.result as string;
        setAvatarImage(imageData);
        console.log('[SettingsView] Avatar image uploaded successfully');
      };
      reader.onerror = () => {
        console.error('[SettingsView] Failed to read image file');
      };
      reader.readAsDataURL(file);
    }
  };

  // Update current time every second for time format toggle
  React.useEffect(() => {
    const interval = setInterval(() => {
      setCurrentTime(new Date());
    }, 1000);

    return () => clearInterval(interval);
  }, []);

  // FEATURE-017: Load calendar connection statuses on mount
  const loadCalendarStatuses = React.useCallback(async () => {
    try {
      const statuses = await calendarService.getStatus();
      setCalendarStatuses(statuses);

      // Update legacy state for backward compatibility
      const googleStatus = statuses.find((s) => s.provider === 'google');
      if (googleStatus?.connected) {
        setConnectedIntegrations((prev) => ({ ...prev, 'google-calendar': true }));
        setCalendarEmail(googleStatus.email ?? null);
      }
    } catch (error) {
      console.error('Failed to load calendar statuses:', error);
    }
  }, []);

  React.useEffect(() => {
    void loadCalendarStatuses();
  }, [loadCalendarStatuses]);

  // Auto-refresh calendar status when window gains focus (after OAuth in browser)
  React.useEffect(() => {
    const handleWindowFocus = () => {
      console.warn('[SettingsView] Window focused - refreshing calendar statuses');
      void loadCalendarStatuses();
    };

    window.addEventListener('focus', handleWindowFocus);
    return () => window.removeEventListener('focus', handleWindowFocus);
  }, [loadCalendarStatuses]);

  // FEATURE-020: Load SAP connection status on mount
  React.useEffect(() => {
    const loadSapStatus = async () => {
      try {
        const isAuthenticated = await SapService.isAuthenticated();
        setConnectedIntegrations((prev) => ({ ...prev, 'sap-s4hana': isAuthenticated }));
      } catch (error) {
        console.error('Failed to load SAP status:', error);
      }
    };

    void loadSapStatus();
  }, []);

  // FEATURE-017: Multi-provider calendar handlers
  const handleConnectCalendar = React.useCallback(async (provider: 'google' | 'microsoft') => {
    console.warn(`[SettingsView] handleConnectCalendar called for provider: ${provider}`);
    const setLoading = provider === 'google' ? setIsLoadingGoogle : setIsLoadingMicrosoft;
    setLoading(true);
    try {
      console.warn(`[SettingsView] Calling calendarService.connect for ${provider}`);
      await calendarService.connect(provider);

      // Poll for connection status (check more frequently at first)
      let pollCount = 0;
      const pollStatus = setInterval(() => {
        void (async () => {
          pollCount++;
          const statuses = await calendarService.getStatus();
          setCalendarStatuses(statuses);

          const providerStatus = statuses.find((s) => s.provider === provider);
          if (providerStatus?.connected) {
            clearInterval(pollStatus);
            console.warn(
              `[SettingsView] ${provider} calendar connected successfully after ${pollCount} polls`
            );

            // Update legacy state for backward compatibility
            if (provider === 'google') {
              setConnectedIntegrations((prev) => ({ ...prev, 'google-calendar': true }));
              setCalendarEmail(providerStatus.email ?? null);
            }

            // Automatically sync calendar entries after successful connection
            console.warn(`[SettingsView] Auto-syncing ${provider} calendar entries...`);
            try {
              const count = await calendarService.syncProvider(provider);
              console.warn(`[SettingsView] Auto-sync complete: ${count} suggestions generated`);
            } catch (syncError) {
              console.error(`[SettingsView] Auto-sync failed:`, syncError);
            } finally {
              setLoading(false);
            }
          }
        })();
      }, 500); // Poll every 500ms instead of 1000ms for faster detection

      // Timeout after 5 minutes
      setTimeout(() => {
        clearInterval(pollStatus);
        setLoading(false);
        console.warn(`[SettingsView] ${provider} calendar connection timed out after 5 minutes`);
      }, 300000);
    } catch (error) {
      console.error(`Failed to connect ${provider} calendar:`, error);
      setLoading(false);
    }
  }, []);

  const handleDisconnectCalendar = React.useCallback(async (provider: 'google' | 'microsoft') => {
    const setLoading = provider === 'google' ? setIsLoadingGoogle : setIsLoadingMicrosoft;
    setLoading(true);
    try {
      await calendarService.disconnect(provider);

      // Refresh statuses
      const statuses = await calendarService.getStatus();
      setCalendarStatuses(statuses);

      // Update legacy state for backward compatibility
      if (provider === 'google') {
        setConnectedIntegrations((prev) => ({ ...prev, 'google-calendar': false }));
        setCalendarEmail(null);
      }
    } catch (error) {
      console.error(`Failed to disconnect ${provider} calendar:`, error);
    } finally {
      setLoading(false);
    }
  }, []);

  // Generate user initials from first and last name
  const getUserInitials = () => {
    if (!userProfile) return 'U';
    const firstInitial = userProfile.first_name?.charAt(0).toUpperCase() || '';
    const lastInitial = userProfile.last_name?.charAt(0).toUpperCase() || '';
    return firstInitial && lastInitial ? `${firstInitial}${lastInitial}` : 'U';
  };

  // FEATURE-016: Check Main API auth status on mount
  React.useEffect(() => {
    const checkAuthStatus = async () => {
      console.log('[SettingsView] Checking auth status on mount...');
      try {
        const status = await WebApiService.getAuthStatus();
        console.log('[SettingsView] Auth status:', status);
        setIsSignedIn(status.authenticated);
        setUserEmail(status.userEmail ?? null);
      } catch (error) {
        console.error('[SettingsView] Failed to check auth status:', error);
        // Set to false so UI shows sign-in button
        setIsSignedIn(false);
      }
    };
    void checkAuthStatus();
  }, []);

  const handleSignIn = async () => {
    console.log('[SettingsView] handleSignIn called');
    setIsSigningIn(true);
    try {
      console.log('[SettingsView] Calling WebApiService.startLogin()');
      const authUrl = await WebApiService.startLogin();
      console.log('[SettingsView] Browser opened to:', authUrl);

      // Poll for auth status change
      const pollInterval = setInterval(() => {
        void (async () => {
          const authenticated = await WebApiService.isAuthenticated();
          if (authenticated) {
            clearInterval(pollInterval);
            const status = await WebApiService.getAuthStatus();
            setIsSignedIn(true);
            setUserEmail(status.userEmail ?? null);
            setIsSigningIn(false);
            console.log('[SettingsView] Sign in successful');
          }
        })();
      }, 1000);

      // Timeout after 5 minutes
      setTimeout(() => {
        clearInterval(pollInterval);
        setIsSigningIn(false);
        console.log('[SettingsView] Sign in timed out after 5 minutes');
      }, 300000);
    } catch (error) {
      console.error('[SettingsView] Sign in failed:', error);
      alert(`Sign in failed: ${error}`);
      setIsSigningIn(false);
    }
  };

  // Load calendar connection status on mount
  React.useEffect(() => {
    const loadCalendarStatus = async () => {
      try {
        const statuses = await calendarService.getStatus();
        // FEATURE-017: Find Google provider in array
        const googleStatus = statuses.find((s) => s.provider === 'google');
        if (googleStatus) {
          setConnectedIntegrations((prev) => ({
            ...prev,
            'google-calendar': googleStatus.connected,
          }));
          setCalendarEmail(googleStatus.email);
        }
      } catch (error) {
        console.error('Failed to load calendar status:', error);
      }
    };
    void loadCalendarStatus();
  }, []);

  // Save settings whenever they change
  React.useEffect(() => {
    const settings = {
      autoApply,
      notifications,
      confidence: confidence[0] ?? 75,
      integrations: connectedIntegrations,
      timeFormat,
    };
    console.warn('💾 [SettingsView] Saving settings to localStorage:', {
      timeFormat,
      fullSettings: settings,
    });
    settingsService.saveSettings(settings);
    // Invalidate the time format cache so it picks up the new setting
    invalidateTimeFormatCache();
  }, [autoApply, notifications, confidence, connectedIntegrations, timeFormat]);

  const toggleIntegration = async (id: string) => {
    // Handle Google Calendar with real OAuth flow
    if (id === 'google-calendar') {
      const isCurrentlyConnected = connectedIntegrations[id];

      if (isCurrentlyConnected) {
        // Disconnect
        if (!calendarEmail) return;
        setIsLoadingGoogle(true);
        try {
          await calendarService.disconnect('google'); // FEATURE-017: provider parameter
          setConnectedIntegrations((prev) => ({
            ...prev,
            [id]: false,
          }));
          setCalendarEmail(null);
        } catch (error) {
          console.error('Failed to disconnect calendar:', error);
        } finally {
          setIsLoadingGoogle(false);
        }
      } else {
        // Connect with OAuth
        setIsLoadingGoogle(true);
        try {
          await calendarService.connect('google'); // FEATURE-017: provider parameter
          // Poll for connection status
          const pollStatus = setInterval(() => {
            void (async () => {
              const statuses = await calendarService.getStatus();
              // FEATURE-017: Find Google provider in array
              const googleStatus = statuses.find((s) => s.provider === 'google');
              if (googleStatus?.connected) {
                setConnectedIntegrations((prev) => ({
                  ...prev,
                  [id]: true,
                }));
                setCalendarEmail(googleStatus.email);
                setIsLoadingGoogle(false);
                clearInterval(pollStatus);
              }
            })();
          }, 1000);

          // Timeout after 5 minutes
          setTimeout(() => {
            clearInterval(pollStatus);
            setIsLoadingGoogle(false);
          }, 300000);
        } catch (error) {
          console.error('Failed to connect calendar:', error);
          setIsLoadingGoogle(false);
        }
      }
    } else if (id === 'sap-s4hana') {
      // FEATURE-020: Handle SAP S/4HANA with OAuth flow
      const isCurrentlyConnected = connectedIntegrations[id];

      if (isCurrentlyConnected) {
        // Disconnect
        setIsLoadingSap(true);
        try {
          await SapService.logout();
          setConnectedIntegrations((prev) => ({
            ...prev,
            [id]: false,
          }));
        } catch (error) {
          console.error('Failed to disconnect SAP:', error);
        } finally {
          setIsLoadingSap(false);
        }
      } else {
        // Connect with OAuth
        setIsLoadingSap(true);
        try {
          await SapService.startLogin();
          // Note: Actual connection completion happens via 'sap-oauth-callback' event
          // The event handler will update connectedIntegrations state
          // For now, keep loading state until callback received

          // Poll for connection status (similar to calendar pattern)
          const pollStatus = setInterval(() => {
            void (async () => {
              const isAuthenticated = await SapService.isAuthenticated();
              if (isAuthenticated) {
                setConnectedIntegrations((prev) => ({
                  ...prev,
                  [id]: true,
                }));
                setIsLoadingSap(false);
                clearInterval(pollStatus);
              }
            })();
          }, 1000);

          // Timeout after 5 minutes
          setTimeout(() => {
            clearInterval(pollStatus);
            setIsLoadingSap(false);
          }, 300000);
        } catch (error) {
          console.error('Failed to connect SAP:', error);
          setIsLoadingSap(false);
        }
      }
    } else {
      // Mock toggle for other integrations
      setConnectedIntegrations((prev) => ({
        ...prev,
        [id]: !prev[id],
      }));
    }
  };

  const getIntegrationIcon = (iconType: string) => {
    switch (iconType) {
      case 'teams':
        return (
          <img
            src="https://upload.wikimedia.org/wikipedia/commons/c/c9/Microsoft_Office_Teams_%282018%E2%80%93present%29.svg"
            alt="Microsoft Teams"
            className="w-7 h-7"
          />
        );
      case 'outlook':
        return (
          <img
            src="https://mailmeteor.com/logos/assets/PNG/Microsoft_Office_Outlook_Logo_512px.png"
            alt="Outlook"
            className="w-7 h-7"
          />
        );
      case 'google-calendar':
        return (
          <img
            src="https://upload.wikimedia.org/wikipedia/commons/a/a5/Google_Calendar_icon_%282020%29.svg"
            alt="Google Calendar"
            className="w-7 h-7"
          />
        );
      case 'sap-s4hana':
        return (
          <img
            src="https://upload.wikimedia.org/wikipedia/commons/5/59/SAP_2011_logo.svg"
            alt="SAP S/4HANA"
            className="w-7 h-7"
          />
        );
      default:
        return <span className="text-2xl">{iconType}</span>;
    }
  };

  const handleSyncCalendar = async () => {
    setIsSyncingCalendar(true);
    try {
      const count = await calendarService.syncNow();
      console.log(`[SettingsView] Calendar synced: ${count} suggestions generated`);
    } catch (error) {
      console.error('[SettingsView] Failed to sync calendar:', error);
    } finally {
      setIsSyncingCalendar(false);
    }
  };

  const handleClearDataClick = () => {
    console.log('[SettingsView] Clear data button clicked');
    setShowClearConfirm(true);
  };

  const handleClearDataConfirm = async () => {
    console.log('[SettingsView] Clearing data...');
    setIsClearingData(true);
    setShowClearConfirm(false);

    try {
      await adminService.clearAllData();
      console.log('[SettingsView] Data cleared successfully');
    } catch (error) {
      console.error('[SettingsView] Failed to clear data:', error);
    } finally {
      setIsClearingData(false);
    }
  };

  const handleClearDataCancel = () => {
    console.log('[SettingsView] Clear data cancelled');
    setShowClearConfirm(false);
  };

  return (
    <div className="backdrop-blur-[24px] overflow-hidden h-full flex flex-col">
      {/* Drag handle bar */}
      <div
        data-tauri-drag-region
        className="flex items-center justify-center py-2 cursor-move rounded-t-[40px] select-none"
      >
        <GripHorizontal className="w-8 h-3 text-gray-400/50 dark:text-gray-500/50 pointer-events-none" />
      </div>

      {/* Header */}
      <div className="p-4 pt-2 border-b border-white/10 dark:border-white/5">
        <div className="flex items-center justify-between">
          <div className="flex items-center gap-2">
            <Button
              variant="ghost"
              size="icon"
              onClick={onBack}
              className="h-7 w-7 text-gray-700 dark:text-gray-300 hover:bg-white/20 dark:hover:bg-white/10"
            >
              <ArrowLeft className="w-3.5 h-3.5" />
            </Button>
            <h2 className="text-sm text-gray-900 dark:text-gray-100">Settings</h2>
          </div>
        </div>
      </div>

      <Tabs defaultValue="account" className="flex-1 flex flex-col overflow-hidden">
        <div className="px-5 pt-3 pb-2 shrink-0">
          <TabsList className="w-full bg-white/10 dark:bg-white/5 backdrop-blur-xl">
            <TabsTrigger value="account" className="flex-1 text-xs">
              <User className="w-3.5 h-3.5" />
              Account
            </TabsTrigger>
            <TabsTrigger value="activity" className="flex-1 text-xs">
              <Settings className="w-3.5 h-3.5" />
              General
            </TabsTrigger>
            <TabsTrigger value="integrations" className="flex-1 text-xs">
              <Plug className="w-3.5 h-3.5" />
              Integrations
            </TabsTrigger>
          </TabsList>
        </div>

        <ScrollArea className="flex-1 rounded-b-[2.5rem] overflow-hidden">
          <div className="px-5 pt-3 pb-8">
            {/* Account Tab */}
            <TabsContent value="account" className="mt-0 space-y-5">
              {/* Profile card - Business card style or skeleton */}
              <div className="bg-white/10 dark:bg-white/5 border border-white/30 dark:border-white/20 rounded-xl p-5">
                {!isSignedIn ? (
                  /* Skeleton state when not signed in */
                  <div className="flex flex-col items-center justify-center py-4 text-center">
                    <div className="relative h-16 w-16 mb-3">
                      <Avatar className="h-full w-full ring-2 ring-white/30 dark:ring-white/20">
                        <AvatarFallback className="bg-white/20 dark:bg-white/10 text-gray-500 dark:text-gray-400">
                          <User className="w-8 h-8" />
                        </AvatarFallback>
                      </Avatar>
                    </div>
                    <h3 className="text-sm font-semibold text-gray-700 dark:text-gray-300 mb-1.5">
                      Sign in to view profile
                    </h3>
                    <p className="text-xs text-gray-600 dark:text-gray-400 mb-3 max-w-xs">
                      Connect your account to access your profile information and sync your data
                    </p>
                    <Button
                      size="sm"
                      onClick={() => void handleSignIn()}
                      disabled={isSigningIn}
                      className="text-xs hover:bg-white/20 dark:hover:bg-white/20 transition-colors bg-white/10 dark:bg-white/10 border border-white/20 dark:border-white/20 text-gray-700 dark:text-gray-300"
                    >
                      {isSigningIn ? (
                        <>
                          <LogIn className="w-3 h-3 mr-1.5 animate-pulse" />
                          Signing in...
                        </>
                      ) : (
                        <>
                          <LogIn className="w-3 h-3 mr-1.5" />
                          Sign In
                        </>
                      )}
                    </Button>
                  </div>
                ) : (
                  /* Profile card when signed in */
                  <div className="relative">
                    {/* Sign out button - top right corner */}
                    <Button
                      variant="ghost"
                      size="icon"
                      onClick={() => {
                        void (async () => {
                          try {
                            console.log('[SettingsView] Signing out...');
                            await WebApiService.logout();
                            setIsSignedIn(false);
                            console.log('[SettingsView] Sign out successful');
                          } catch (error) {
                            console.error('[SettingsView] Failed to sign out:', error);
                          }
                        })();
                      }}
                      className="absolute -top-1 -right-1 h-6 w-6 rounded-full bg-white/20 dark:bg-white/10 hover:bg-red-500/20 dark:hover:bg-red-500/20 border border-white/30 dark:border-white/20 hover:border-red-500/50 transition-all z-10"
                      title="Sign out"
                    >
                      <LogIn className="w-3 h-3 text-gray-600 dark:text-gray-400 hover:text-red-600 dark:hover:text-red-400 rotate-180" />
                    </Button>

                    <div className="flex items-start gap-4">
                      {/* Left: Avatar with camera button or dropdown */}
                      <div className="relative shrink-0">
                        {/* Hidden file input */}
                        <input
                          ref={fileInputRef}
                          type="file"
                          accept="image/*"
                          onChange={handleFileChange}
                          className="hidden"
                        />

                        {avatarImage ? (
                          /* Avatar with photo - show dropdown on hover */
                          <DropdownMenu>
                            <DropdownMenuTrigger asChild>
                              <button className="relative h-20 w-20 rounded-full cursor-pointer group ring-2 ring-white/30 dark:ring-white/20 hover:ring-white/50 dark:hover:ring-white/40 transition-all">
                                <Avatar className="h-full w-full">
                                  <img
                                    src={avatarImage}
                                    alt="Profile"
                                    className="h-full w-full object-cover rounded-full"
                                  />
                                </Avatar>
                                {/* Hover overlay */}
                                <div className="absolute inset-0 bg-black/40 rounded-full opacity-0 group-hover:opacity-100 transition-opacity flex items-center justify-center">
                                  <Camera className="w-5 h-5 text-white" />
                                </div>
                              </button>
                            </DropdownMenuTrigger>
                            <DropdownMenuContent align="start" className="w-40">
                              <DropdownMenuItem onClick={handlePhotoUpload}>
                                <Camera className="w-4 h-4 mr-2" />
                                Replace photo
                              </DropdownMenuItem>
                              <DropdownMenuItem
                                onClick={handleRemovePhoto}
                                className="text-red-600 dark:text-red-400"
                              >
                                <X className="w-4 h-4 mr-2" />
                                Remove photo
                              </DropdownMenuItem>
                            </DropdownMenuContent>
                          </DropdownMenu>
                        ) : (
                          /* Avatar without photo - show camera button */
                          <div className="relative h-20 w-20">
                            <Avatar className="h-full w-full ring-2 ring-white/30 dark:ring-white/20">
                              <AvatarFallback className="bg-white/30 dark:bg-white/20 text-gray-700 dark:text-gray-300 text-2xl font-semibold">
                                {getUserInitials()}
                              </AvatarFallback>
                            </Avatar>
                            {/* Camera button overlay */}
                            <Button
                              variant="ghost"
                              size="icon"
                              onClick={handlePhotoUpload}
                              className="absolute -bottom-1 -right-1 h-6 w-6 rounded-full bg-white/30 dark:bg-white/20 hover:bg-white/40 dark:hover:bg-white/30 hover:scale-110 active:scale-95 border border-white/40 dark:border-white/30 shadow-sm transition-all duration-150 cursor-pointer"
                            >
                              <Camera className="w-3 h-3 text-gray-700 dark:text-gray-300" />
                            </Button>
                          </div>
                        )}
                      </div>

                      {/* Right: User info - Business card style */}
                      <div className="flex-1 min-w-0">
                        {userProfile && (
                          <>
                            <h3 className="text-lg font-semibold text-gray-900 dark:text-gray-100 mb-0.5">
                              {userProfile.first_name || ''} {userProfile.last_name || ''}
                            </h3>
                            {userProfile.title && (
                              <p className="text-sm text-gray-600 dark:text-gray-400 mb-1">
                                {userProfile.title}
                              </p>
                            )}
                            {userProfile.department && (
                              <p className="text-sm font-medium text-gray-700 dark:text-gray-300 mb-3">
                                {userProfile.department}
                              </p>
                            )}

                            <div className="space-y-1.5">
                              <div className="flex items-center gap-2 text-xs text-gray-600 dark:text-gray-400">
                                <span className="font-medium">Email:</span>
                                <span className="truncate">{userProfile.email}</span>
                              </div>
                              {userProfile.phone_number && (
                                <div className="flex items-center gap-2 text-xs text-gray-600 dark:text-gray-400">
                                  <span className="font-medium">Phone:</span>
                                  <span>{userProfile.phone_number}</span>
                                </div>
                              )}
                              <div className="flex items-center gap-2 text-xs text-gray-600 dark:text-gray-400">
                                <span className="font-medium">Timezone:</span>
                                <span>{userProfile.timezone}</span>
                              </div>
                              {userProfile.location && (
                                <div className="flex items-center gap-2 text-xs text-gray-600 dark:text-gray-400">
                                  <span className="font-medium">Location:</span>
                                  <span>{userProfile.location}</span>
                                </div>
                              )}
                            </div>
                          </>
                        )}
                      </div>
                    </div>
                  </div>
                )}
              </div>

              {/* Help & Tutorials section with dividers - only show when signed in */}
              {onRestartTutorial && isSignedIn && (
                <>
                  <Separator className="bg-white/20 dark:bg-white/10" />

                  <div>
                    <div className="flex items-center gap-2 mb-3">
                      <HelpCircle className="w-4 h-4 text-gray-700 dark:text-gray-300" />
                      <h3 className="text-sm text-gray-900 dark:text-gray-100">Help & Tutorials</h3>
                    </div>

                    <div className="space-y-2">
                      <Button
                        variant="outline"
                        size="sm"
                        onClick={onRestartTutorial}
                        className="w-full text-xs backdrop-blur-xl bg-white/20 dark:bg-white/10 border border-white/30 dark:border-white/20 hover:bg-white/30 dark:hover:bg-white/15"
                      >
                        Restart Tutorial
                      </Button>

                      {!showClearConfirm ? (
                        <Button
                          variant="outline"
                          size="sm"
                          onClick={handleClearDataClick}
                          disabled={isClearingData}
                          className="w-full text-xs backdrop-blur-xl bg-red-500/20 dark:bg-red-500/10 border border-red-500/30 dark:border-red-500/20 text-red-700 dark:text-red-400 hover:bg-red-500/30 dark:hover:bg-red-500/15"
                        >
                          {isClearingData ? 'Clearing...' : 'Clear All Local Data'}
                        </Button>
                      ) : (
                        <div className="flex gap-2">
                          <Button
                            variant="outline"
                            size="sm"
                            onClick={() => void handleClearDataConfirm()}
                            disabled={isClearingData}
                            className="flex-1 text-xs bg-red-600 dark:bg-red-600 border-red-700 dark:border-red-700 text-white hover:bg-red-700 dark:hover:bg-red-700"
                          >
                            Confirm
                          </Button>
                          <Button
                            variant="outline"
                            size="sm"
                            onClick={handleClearDataCancel}
                            disabled={isClearingData}
                            className="flex-1 text-xs backdrop-blur-xl bg-white/20 dark:bg-white/10 border border-white/30 dark:border-white/20 hover:bg-white/30 dark:hover:bg-white/15"
                          >
                            Cancel
                          </Button>
                        </div>
                      )}
                    </div>
                  </div>
                </>
              )}
            </TabsContent>

            {/* Activity Tab */}
            <TabsContent value="activity" className="mt-0 space-y-4">
              {/* Tracking Settings Card */}
              <div className="bg-white/10 dark:bg-white/5 border border-white/30 dark:border-white/20 rounded-xl p-4 space-y-4">
                <div className="flex items-center gap-2 pb-2 border-b border-white/20 dark:border-white/10">
                  <Activity className="w-4 h-4 text-gray-700 dark:text-gray-300" />
                  <h3 className="text-sm text-gray-900 dark:text-gray-100 font-medium">Tracking</h3>
                </div>

                {/* Idle Detection */}
                <IdleDetectionSettings />
              </div>

              {/* Suggestions Card */}
              <div className="bg-white/10 dark:bg-white/5 border border-white/30 dark:border-white/20 rounded-xl p-4 space-y-4">
                <div className="flex items-center gap-2 pb-2 border-b border-white/20 dark:border-white/10">
                  <Brain className="w-4 h-4 text-gray-700 dark:text-gray-300" />
                  <h3 className="text-sm text-gray-900 dark:text-gray-100 font-medium">
                    Suggestions
                  </h3>
                </div>

                {/* Auto-apply suggestions with confidence threshold */}
                <div className="space-y-3">
                  <div className="flex items-center justify-between">
                    <div className="flex-1">
                      <Label
                        htmlFor="auto-apply"
                        className="text-sm text-gray-700 dark:text-gray-300"
                      >
                        Auto-apply suggestions
                      </Label>
                      <p className="text-xs text-gray-500 dark:text-gray-400 mt-0.5">
                        Automatically apply AI suggestions above threshold
                      </p>
                    </div>
                    <Switch id="auto-apply" checked={autoApply} onCheckedChange={setAutoApply} />
                  </div>

                  {/* Confidence threshold slider and notifications - only show if auto-apply is enabled */}
                  {autoApply && (
                    <>
                      <div>
                        <div className="relative">
                          {/* Dynamic gradient background based on threshold */}
                          <div className="absolute inset-0 h-4 rounded-full overflow-hidden pointer-events-none">
                            <div
                              className={`w-full h-full opacity-30 ${
                                (confidence[0] ?? 75) >= 80
                                  ? 'bg-gradient-to-r from-green-400 via-green-500 to-green-600'
                                  : (confidence[0] ?? 75) >= 70
                                    ? 'bg-gradient-to-r from-yellow-400 via-yellow-500 to-yellow-600'
                                    : 'bg-gradient-to-r from-red-400 via-red-500 to-red-600'
                              }`}
                            />
                          </div>
                          {/* Slider on top with matching gradient */}
                          <Slider
                            value={confidence}
                            onValueChange={setConfidence}
                            min={50}
                            max={100}
                            step={5}
                            className={`w-full relative [&_[data-slot=slider-track]]:bg-transparent ${
                              (confidence[0] ?? 75) >= 80
                                ? '[&_[data-slot=slider-range]]:bg-gradient-to-r [&_[data-slot=slider-range]]:from-green-400 [&_[data-slot=slider-range]]:via-green-500 [&_[data-slot=slider-range]]:to-green-600'
                                : (confidence[0] ?? 75) >= 70
                                  ? '[&_[data-slot=slider-range]]:bg-gradient-to-r [&_[data-slot=slider-range]]:from-yellow-400 [&_[data-slot=slider-range]]:via-yellow-500 [&_[data-slot=slider-range]]:to-yellow-600'
                                  : '[&_[data-slot=slider-range]]:bg-gradient-to-r [&_[data-slot=slider-range]]:from-red-400 [&_[data-slot=slider-range]]:via-red-500 [&_[data-slot=slider-range]]:to-red-600'
                            }`}
                          />
                        </div>
                        <div className="flex items-center justify-between mt-2">
                          <span className="text-sm text-gray-900 dark:text-gray-100 font-medium">
                            {confidence[0]}%
                          </span>
                        </div>
                      </div>

                      {/* Enable notifications */}
                      <div className="flex items-center justify-between pt-3 border-t border-white/10 dark:border-white/5">
                        <div className="flex-1">
                          <Label
                            htmlFor="notifications"
                            className="text-sm text-gray-700 dark:text-gray-300"
                          >
                            Enable notifications
                          </Label>
                          <p className="text-xs text-gray-500 dark:text-gray-400 mt-0.5">
                            Get notified when suggestions are applied
                          </p>
                        </div>
                        <Switch
                          id="notifications"
                          checked={notifications}
                          onCheckedChange={setNotifications}
                        />
                      </div>
                    </>
                  )}
                </div>
              </div>

              {/* Time Format Card */}
              <div className="bg-white/10 dark:bg-white/5 border border-white/30 dark:border-white/20 rounded-xl p-4">
                <div className="flex items-center gap-2 pb-2 border-b border-white/20 dark:border-white/10">
                  <Clock className="w-4 h-4 text-gray-700 dark:text-gray-300" />
                  <h3 className="text-sm text-gray-900 dark:text-gray-100 font-medium">
                    Time Format
                  </h3>
                </div>

                {/* Time Format Toggle - Single button showing actual time */}
                <div className="flex items-center justify-between mt-4">
                  <div className="flex-1">
                    <Label
                      htmlFor="time-format"
                      className="text-sm text-gray-700 dark:text-gray-300"
                    >
                      Display format
                    </Label>
                    <p className="text-xs text-gray-500 dark:text-gray-400 mt-0.5">
                      Click to switch between 12-hour and 24-hour format
                    </p>
                  </div>
                  <Button
                    variant="outline"
                    size="sm"
                    onClick={() => setTimeFormat(timeFormat === '12h' ? '24h' : '12h')}
                    className="text-sm px-4 h-9 tabular-nums tracking-tight backdrop-blur-xl bg-white/20 dark:bg-white/10 border border-white/30 dark:border-white/20 hover:bg-white/30 dark:hover:bg-white/15 transition-all"
                  >
                    {formatTime(currentTime, timeFormat)}
                  </Button>
                </div>
              </div>
            </TabsContent>

            {/* Integrations Tab */}
            <TabsContent value="integrations" className="mt-0">
              <div className="space-y-3">
                {/* FEATURE-017: Multi-provider calendar integration */}
                <div className="mb-4">
                  <h3 className="text-sm text-gray-900 dark:text-gray-100 font-medium mb-3">
                    Calendar Integration
                  </h3>
                  <CalendarProviderCard
                    provider="google"
                    status={calendarStatuses.find((s) => s.provider === 'google')}
                    onConnect={() => void handleConnectCalendar('google')}
                    onDisconnect={() => void handleDisconnectCalendar('google')}
                    isLoading={isLoadingGoogle}
                  />
                  <CalendarProviderCard
                    provider="microsoft"
                    status={calendarStatuses.find((s) => s.provider === 'microsoft')}
                    onConnect={() => void handleConnectCalendar('microsoft')}
                    onDisconnect={() => void handleDisconnectCalendar('microsoft')}
                    isLoading={isLoadingMicrosoft}
                  />
                </div>

                {/* Legacy integrations (exclude google-calendar, handled by CalendarProviderCard above) */}
                {integrations
                  .filter((integration) => integration.id !== 'google-calendar')
                  .map((integration) => (
                    <div
                      key={integration.id}
                      className="flex items-center justify-between bg-white/10 dark:bg-white/5 border border-white/30 dark:border-white/20 rounded-xl p-3"
                    >
                      <div className="flex items-center gap-3">
                        {getIntegrationIcon(integration.icon)}
                        <div>
                          <div className="text-sm text-gray-900 dark:text-gray-100">
                            {integration.name}
                          </div>
                          <div className="text-xs text-gray-500 dark:text-gray-400">
                            {connectedIntegrations[integration.id]
                              ? integration.id === 'google-calendar' && calendarEmail
                                ? calendarEmail
                                : 'Connected'
                              : 'Not connected'}
                          </div>
                        </div>
                      </div>
                      {integration.id === 'google-calendar' &&
                      connectedIntegrations[integration.id] ? (
                        <DropdownMenu>
                          <DropdownMenuTrigger asChild>
                            <Button
                              size="sm"
                              variant="outline"
                              disabled={isLoadingGoogle || isSyncingCalendar}
                              className="text-xs h-7 bg-green-500/20 hover:bg-green-500/30 dark:bg-green-400/20 dark:hover:bg-green-400/30 border border-green-500/30 dark:border-green-400/30 text-green-400 dark:text-green-400"
                            >
                              <Check className="w-3 h-3 mr-1" />
                              Connected
                              <ChevronDown className="w-3 h-3 ml-1" />
                            </Button>
                          </DropdownMenuTrigger>
                          <DropdownMenuContent align="end" className="w-40">
                            <DropdownMenuItem
                              onClick={() => void handleSyncCalendar()}
                              disabled={isSyncingCalendar}
                            >
                              <RefreshCw
                                className={`w-4 h-4 mr-2 ${isSyncingCalendar ? 'animate-spin' : ''}`}
                              />
                              {isSyncingCalendar ? 'Syncing...' : 'Sync Now'}
                            </DropdownMenuItem>
                            <DropdownMenuItem
                              onClick={() => void handleDisconnectCalendar('google')}
                              disabled={isLoadingGoogle}
                              className="text-red-600 dark:text-red-400 focus:text-red-600 dark:focus:text-red-400"
                            >
                              <X className="w-4 h-4 mr-2" />
                              Disconnect
                            </DropdownMenuItem>
                          </DropdownMenuContent>
                        </DropdownMenu>
                      ) : (
                        <Button
                          size="sm"
                          variant={connectedIntegrations[integration.id] ? 'outline' : 'default'}
                          onClick={() => void toggleIntegration(integration.id)}
                          disabled={
                            (integration.id === 'google-calendar' && isLoadingGoogle) ||
                            (integration.id === 'sap-s4hana' && isLoadingSap)
                          }
                          className={
                            connectedIntegrations[integration.id]
                              ? 'text-xs h-7 bg-green-500/20 hover:bg-green-500/30 dark:bg-green-400/20 dark:hover:bg-green-400/30 border border-green-500/30 dark:border-green-400/30 text-green-400 dark:text-green-400'
                              : 'text-xs h-7 backdrop-blur-xl bg-white/20 dark:bg-white/10 border border-white/30 dark:border-white/20 hover:bg-white/30 dark:hover:bg-white/15 text-gray-700 dark:text-gray-300'
                          }
                        >
                          {(integration.id === 'google-calendar' && isLoadingGoogle) ||
                          (integration.id === 'sap-s4hana' && isLoadingSap) ? (
                            'Connecting...'
                          ) : connectedIntegrations[integration.id] ? (
                            <>
                              <Check className="w-3 h-3 mr-1" />
                              Connected
                            </>
                          ) : (
                            'Connect'
                          )}
                        </Button>
                      )}
                    </div>
                  ))}
              </div>
            </TabsContent>
          </div>
        </ScrollArea>
      </Tabs>
    </div>
  );
}
