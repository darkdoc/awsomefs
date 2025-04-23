
/*
*  hello-1.c - The simplest kernel module.
*/
#include <linux/module.h>	/* Needed by all modules */
#include <linux/kernel.h>	/* Needed for KERN_INFO */

MODULE_LICENSE("GPL");
MODULE_AUTHOR("Bharath Reddy");
MODULE_DESCRIPTION("A simple hello world module");
MODULE_VERSION("0.01");
int init_module(void)
{
    printk(KERN_INFO "Hello world 1.\n");
    /*
     * A non 0 return means init_module failed; module can't be loaded.
     */
    return 0;
}
void cleanup_module(void)
{
    printk(KERN_INFO "Goodbye world 1.\n");
}