// ip shenanigans
// must check that we are within the subnet for middle component
// net, also subnet
// MODULE_ID.SUBMODULE_NET.SYSMODULE ID

// if we are a "middle component, we mas only over module_ID, "

// modules have a basic ID and don't care
// COM modules need to subscribe to everything under the mask


// HUB COM <-> HUB2ADV COM  - IB - ADV2BASIC COM <-> BASIC2ADV COM - IB


//option 1: tunneling: Send a message to hub2adv sending to adv2basic sending to basic2adv sending to module x ... shit
// option 2:  LWIP_HOOK_IP4_ROUTE (dest) -> select netif to write to
/*

#define LWIP_HOOK_ETHARP_GET_GW	(	 	netif,
 	dest 
)		
LWIP_HOOK_ETHARP_GET_GW(netif, dest):



 */